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
    Write-Color "    2. Try to install the VS Code extension" -Color Gray
    Write-Color ""
    Write-Color "  For other editors, you'll need to install the" -Color White
    Write-Color "  WakaTime plugin manually. Need help? Ask here:" -Color White
    Write-Color "  $SlackChannel" -Color Cyan
    Write-Color ""
    Write-Color "============================================================" -Color Yellow
    Write-Color ""

    # --- Step 1: Write .wakatime.cfg ---

    $ConfigPath = Join-Path $env:USERPROFILE ".wakatime.cfg"

    $ConfigContent = @"
[settings]
api_url = $ApiUrl
api_key = $ApiKey
heartbeat_rate_limit_seconds = 30
exclude_unknown_project = true

# help with config: https://github.com/wakatime/wakatime-cli/blob/develop/USAGE.md#ini-config-file
"@

    Write-Color "[1/2] " -Color Green -NoNewline
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

    # --- Step 2: Try to install VS Code extension ---

    Write-Color "[2/2] " -Color Green -NoNewline
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
