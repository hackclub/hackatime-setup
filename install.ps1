#  _                _         _   _                
# | |__   __ _  ___| | ____ _| |_(_)_ __ ___   ___ 
# | '_ \ / _` |/ __| |/ / _` | __| | '_ ` _ \ / _ \
# | | | | (_| | (__|   < (_| | |_| | | | | | |  __/
# |_| |_|\__,_|\___|_|\_\__,_|\__|_|_| |_| |_|\___|
#
# This script downloads the Hackatime installer from our GitHub. It's written in Rust and is
# open source: https://github.com/hackclub/hackatime-setup
#
# If you need help, ask in the #hackatime-v2 channel on Slack!

param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$ApiKey
)

$ErrorActionPreference = "Stop"

$Repo = "hackclub/hackatime-setup"
$BinaryName = "hackatime_setup.exe"

# Detect architecture
$Arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
switch ($Arch) {
    "AMD64" { $ArchName = "x86_64" }
    "x86"   { $ArchName = "i686" }
    "ARM64" { 
        Write-Host "ARM64 Windows not yet supported, trying x86_64 emulation..."
        $ArchName = "x86_64" 
    }
    default { 
        Write-Error "Unsupported architecture: $Arch"
        exit 1
    }
}

$AssetName = "hackatime_setup-windows-${ArchName}.zip"

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

    & (Join-Path $TempDir $BinaryName) --key $ApiKey
}
finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
