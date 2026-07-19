param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ApiKey,

    [Parameter(Mandatory = $false, Position = 1)]
    [string]$ApiUrl,

    [Parameter(Mandatory = $false)]
    [Alias("y")]
    [switch]$Yes
)

$ErrorActionPreference = "Stop"

$Repo = "hackclub/hackatime-setup"
$BinaryName = "hackatime_cli.exe"

$AssetName = "hackatime_setup-windows-x86_64.zip"

$DefaultApiUrl = "https://hackatime.hackclub.com/api/hackatime/v1"
if (-not $ApiUrl) {
    $ApiUrl = $DefaultApiUrl
}

$SlackChannel = "https://hackclub.enterprise.slack.com/archives/C0AFG0XGGMP"

# --- Helper functions for colored output ---

function Write-Color {
    param(
        [string]$Text,
        [ConsoleColor]$Color = "White",
        [switch]$NoNewline
    )

    $prev = $Host.UI.RawUI.ForegroundColor
    $Host.UI.RawUI.ForegroundColor = $Color

    if ($NoNewline) {
        Write-Host $Text -NoNewline
    } else {
        Write-Host $Text
    }

    $Host.UI.RawUI.ForegroundColor = $prev
}

function Write-Banner {
    Write-Color ""
    Write-Color "  _                _         _   _                " -Color Cyan
    Write-Color " | |__   __ _  ___| | ____ _| |_(_)_ __ ___   ___ " -Color Cyan
    Write-Color " | '_ \ / _` |/ __| |/ / _` | __| | '_ `` _ \ / _ \" -Color Cyan
    Write-Color " | | | | (_| | (__|   < (_| | |_| | | | | | |  __/" -Color Cyan
    Write-Color " |_| |_|\__,_|\___|_|\_\__,_|\__|_|_| |_| |_|\___|" -Color Cyan
    Write-Color ""
}

function Get-WakaTimeHome {
    if ($env:WAKATIME_HOME) {
        return $env:WAKATIME_HOME
    }

    return $env:USERPROFILE
}

function Get-WakaTimeInternalConfigPath {
    if ($env:WAKATIME_HOME) {
        return Join-Path $env:WAKATIME_HOME "wakatime-internal.cfg"
    }

    return Join-Path (Join-Path $env:USERPROFILE ".wakatime") "wakatime-internal.cfg"
}

function Set-InternalConfigValue {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Value
    )

    $lines = @()
    if (Test-Path $Path) {
        $lines = @(Get-Content -Path $Path -ErrorAction Stop)
    }

    $updated = New-Object System.Collections.Generic.List[string]
    $inInternal = $false
    $sawInternal = $false
    $wroteKey = $false

    foreach ($line in $lines) {
        if ($line -match '^\s*\[(.+)\]\s*$') {
            if ($inInternal -and -not $wroteKey) {
                $updated.Add("ai_logs_last_parsed_at = $Value")
                $wroteKey = $true
            }

            $section = $Matches[1].Trim()
            $inInternal = $section -ieq "internal"
            if ($inInternal) {
                $sawInternal = $true
            }

            $updated.Add($line)
            continue
        }

        if ($inInternal -and $line -match '^\s*ai_logs_last_parsed_at\s*=') {
            if (-not $wroteKey) {
                $updated.Add("ai_logs_last_parsed_at = $Value")
                $wroteKey = $true
            }
            continue
        }

        $updated.Add($line)
    }

    if ($inInternal -and -not $wroteKey) {
        $updated.Add("ai_logs_last_parsed_at = $Value")
        $wroteKey = $true
    }

    if (-not $sawInternal) {
        if ($updated.Count -gt 0 -and $updated[$updated.Count - 1] -ne "") {
            $updated.Add("")
        }
        $updated.Add("[internal]")
        $updated.Add("ai_logs_last_parsed_at = $Value")
    }

    Set-Content -Path $Path -Value $updated -Encoding UTF8 -ErrorAction Stop
}

# --- Simplified setup ---

function Install-Simplified {
    Write-Banner

    Write-Color "============================================================" -Color Yellow
    Write-Color "  SETUP" -Color Yellow
    Write-Color "============================================================" -Color Yellow
    Write-Color ""
    Write-Color "  Setting up Hackatime on this system." -Color White
    Write-Color "  This will:" -Color White
    Write-Color "    1. Write your ~/.wakatime.cfg config file" -Color Gray
    Write-Color "    2. Mark existing AI activity as already checked" -Color Gray
    Write-Color "    3. Try to install the VS Code extension" -Color Gray
    Write-Color ""
    Write-Color "  For other editors, you'll need to install the" -Color White
    Write-Color "  WakaTime plugin manually. Need help? Ask here:" -Color White
    Write-Color "  $SlackChannel" -Color Cyan
    Write-Color ""
    Write-Color "============================================================" -Color Yellow
    Write-Color ""

    # --- Step 1: Write .wakatime.cfg ---

    $ConfigPath = Join-Path (Get-WakaTimeHome) ".wakatime.cfg"

    $ConfigContent = @"
[settings]
api_url = $ApiUrl
api_key = $ApiKey
heartbeat_rate_limit_seconds = 30
exclude_unknown_project = true

# help with config: https://github.com/wakatime/wakatime-cli/blob/develop/USAGE.md#ini-config-file
"@

    Write-Color "[1/3] " -Color Green -NoNewline
    Write-Color "Writing config to " -NoNewline
    Write-Color $ConfigPath -Color Green

    try {
        Set-Content -Path $ConfigPath -Value $ConfigContent -Encoding UTF8
        Write-Color "  OK " -Color Green -NoNewline
        Write-Color "Config written successfully."
    } catch {
        Write-Color "  FAIL " -Color Red -NoNewline
        Write-Color "Could not write config: $_"
        return
    }

    Write-Color ""

    # --- Step 2: Write wakatime-internal.cfg ---

    $InternalConfigPath = Get-WakaTimeInternalConfigPath
    $InternalConfigDir = Split-Path -Parent $InternalConfigPath
    $AiLogsLastParsedAt = [System.DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")

    Write-Color "[2/3] " -Color Green -NoNewline
    Write-Color "Writing WakaTime internal config to " -NoNewline
    Write-Color $InternalConfigPath -Color Green

    try {
        New-Item -ItemType Directory -Path $InternalConfigDir -Force | Out-Null
        Set-InternalConfigValue -Path $InternalConfigPath -Value $AiLogsLastParsedAt
        Write-Color "  OK " -Color Green -NoNewline
        Write-Color "Initial AI activity backfill will be skipped."
    } catch {
        Write-Color "  WARN " -Color Yellow -NoNewline
        Write-Color "Could not write internal config to skip initial AI activity backfill: $_"
    }

    Write-Color ""

    # --- Step 3: Try to install VS Code extension ---

    Write-Color "[3/3] " -Color Green -NoNewline
    Write-Color "Checking for VS Code..."

    $VsCodeInstalled = $false
    $CodeCli = $null

    # Try to find the VS Code CLI
    $CliCandidates = @("code")
    foreach ($candidate in $CliCandidates) {
        try {
            $found = Get-Command $candidate -ErrorAction SilentlyContinue
            if ($found) {
                $CodeCli = $found.Source
                break
            }
        } catch {
        }
    }

    # Also check common Windows install paths
    if (-not $CodeCli) {
        $FallbackPaths = @(
            "$env:LOCALAPPDATA\Programs\Microsoft VS Code\bin\code.cmd",
            "$env:ProgramFiles\Microsoft VS Code\bin\code.cmd",
            "${env:ProgramFiles(x86)}\Microsoft VS Code\bin\code.cmd"
        )

        foreach ($path in $FallbackPaths) {
            if (Test-Path $path) {
                $CodeCli = $path
                break
            }
        }
    }

    if ($CodeCli) {
        Write-Color "  Found VS Code, installing Hackatime extension..."
        try {
            $process = Start-Process `
                -FilePath "cmd" `
                -ArgumentList "/C", "`"$CodeCli`"", "--install-extension", "hackatime.hackatime-time-tracker" `
                -Wait `
                -PassThru `
                -NoNewWindow 2>$null

            if ($process.ExitCode -eq 0) {
                $VsCodeInstalled = $true
                Write-Color "  OK " -Color Green -NoNewline
                Write-Color "Hackatime extension installed."
            } else {
                Write-Color "  Note " -Color Yellow -NoNewline
                Write-Color "Extension install exited with code $($process.ExitCode)."
            }
        } catch {
            Write-Color "  Note " -Color Yellow -NoNewline
            Write-Color "Could not install VS Code extension: $_"
        }
    } else {
        Write-Color "  Note " -Color Yellow -NoNewline
        Write-Color "VS Code not found. Install the 'Hackatime Time Tracker' extension manually from the marketplace."
    }

    Write-Color ""

    # --- Summary ---

    Write-Color "============================================================" -Color Green
    Write-Color "  SETUP COMPLETE" -Color Green
    Write-Color "============================================================" -Color Green
    Write-Color ""
    Write-Color "  Config: " -NoNewline
    Write-Color $ConfigPath -Color Green
    Write-Color "  Internal config: " -NoNewline
    Write-Color $InternalConfigPath -Color Green

    if ($VsCodeInstalled) {
        Write-Color "  VS Code: " -NoNewline
        Write-Color "Hackatime extension installed" -Color Green
    } else {
        Write-Color "  VS Code: " -NoNewline
        Write-Color "Install the 'Hackatime Time Tracker' extension manually" -Color Yellow
    }

    Write-Color ""
    Write-Color "  For other editors, install the WakaTime plugin manually." -Color White
    Write-Color "  Docs: " -NoNewline
    Write-Color "https://hackatime.hackclub.com/docs" -Color Cyan
    Write-Color ""
    Write-Color "  Need help? Ask in #hackatime-help on Slack:" -Color White
    Write-Color "  $SlackChannel" -Color Cyan
    Write-Color ""
    Write-Color "  Tip: " -Color Yellow -NoNewline
    Write-Color "Restart your editor after setup for tracking to begin." -Color White
    Write-Color ""
}

# --- Main installer flow ---

# Get latest release
$ReleasesUri = "https://api.github.com/repos/$Repo/releases/latest"
$Release = Invoke-RestMethod -Uri $ReleasesUri -Headers @{ "User-Agent" = "PowerShell" }

$Asset = $Release.assets | Where-Object { $_.name -eq $AssetName }
if (-not $Asset) {
    Write-Error "Could not find release asset: $AssetName"
    exit 1
}

$DownloadUrl = $Asset.browser_download_url

# Download and extract to temp directory
$TempDir = Join-Path $env:TEMP "hackatime_setup_$(Get-Random)"
New-Item -ItemType Directory -Path $TempDir | Out-Null
$ZipPath = Join-Path $TempDir $AssetName

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    $ExePath = Join-Path $TempDir $BinaryName

    # Try running the installer - if anything fails, use simplified setup
    try {
        $ExeArgs = @("--key", $ApiKey)
        if ($ApiUrl -ne $DefaultApiUrl) {
            $ExeArgs += @("--api-url", $ApiUrl)
        }
        if ($Yes) {
            $ExeArgs += "--yes"
        }
        & $ExePath @ExeArgs

        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
            throw "Installer exited with code $LASTEXITCODE"
        }
    } catch {
        Write-Color ""
        Write-Color "Using simplified setup for this system..." -Color Yellow
        Write-Color ""

        Install-Simplified
    }
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
