#!/bin/bash
set -euo pipefail

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
                                                 
REPO="hackclub/hackatime-setup"
BINARY_NAME="hackatime_setup"

# Check for API key argument
if [ $# -lt 1 ]; then
    echo "Usage: $0 <api-key> [api-url]"
    echo "  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | bash -s -- YOUR_API_KEY"
    exit 1
fi

API_KEY="$1"
API_URL="${2:-}"

# Detect OS
OS="$(uname -s)"

if [ -f /etc/NIXOS ]; then
  echo "NixOS detected, running hackatime-setup..."
  # Use nix shell + exec as a workaround
  nix run github:tickreyiz/hackatime-setup -- --key 76ab3aff-99fe-4856-a901-473b77f75cf9
  exit 0
fi
case "$OS" in
    Linux*)  OS_NAME="linux" ;;
    Darwin*) OS_NAME="macos" ;;
    *)       echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)  ARCH_NAME="x86_64" ;;
    arm64|aarch64) ARCH_NAME="aarch64" ;;
    *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ASSET_NAME="hackatime_setup-${OS_NAME}-${ARCH_NAME}.tar.gz"

# Get latest release download URL
DOWNLOAD_URL=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep "browser_download_url.*${ASSET_NAME}" \
    | cut -d '"' -f 4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find release for $ASSET_NAME"
    exit 1
fi

# Download and extract to temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

curl -sL "$DOWNLOAD_URL" -o "$TEMP_DIR/$ASSET_NAME"
tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$TEMP_DIR"
chmod +x "$TEMP_DIR/$BINARY_NAME"

if [ -n "$API_URL" ]; then
    "$TEMP_DIR/$BINARY_NAME" --key "$API_KEY" --api-url "$API_URL"
else
    "$TEMP_DIR/$BINARY_NAME" --key "$API_KEY"
fi
