#!/usr/bin/env bash
# Sign the Windows zip of a GitHub release with Azure Artifact Signing using the
# credentials of whoever is logged in to the Azure CLI, then replace the asset.
#
# Fallback for when the CI service principal cannot sign (see README). Needs:
#   brew install azure-cli jsign osslsigncode   (plus gh, unzip, zip)
#   az login   (an account with the "Artifact Signing Certificate Profile Signer" role)
#
# Usage: scripts/sign-windows-release.sh [tag] [--force]
#   tag      release tag, defaults to the latest release
#   --force  re-sign even if the exe already carries a signature
set -euo pipefail

REPO="hackclub/hackatime-setup"
ASSET="hackatime_setup-windows-x86_64.zip"
EXE="hackatime_cli.exe"
ENDPOINT="eus.codesigning.azure.net"
PROFILE="hackclub/hackatime-desktop" # <signing account>/<certificate profile>
MS_ROOT_URL="https://www.microsoft.com/pkiops/certs/Microsoft%20Identity%20Verification%20Root%20Certificate%20Authority%202020.crt"

TAG=""
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    -*) echo "unknown flag: $arg" >&2; exit 2 ;;
    *) TAG="$arg" ;;
  esac
done

for tool in az jsign osslsigncode gh unzip zip curl openssl; do
  command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 1; }
done

if [[ -z "$TAG" ]]; then
  TAG=$(gh release view --repo "$REPO" --json tagName --jq .tagName)
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

echo "==> Downloading $ASSET from $TAG"
gh release download "$TAG" --repo "$REPO" --pattern "$ASSET"
unzip -q "$ASSET"
[[ -f "$EXE" ]] || { echo "$EXE not found inside $ASSET" >&2; exit 1; }

echo "==> Fetching Microsoft root CA for verification"
curl -fsSL "$MS_ROOT_URL" -o msroot.crt
openssl x509 -inform DER -in msroot.crt -out msroot.pem 2>/dev/null || cp msroot.crt msroot.pem

if osslsigncode verify -CAfile msroot.pem -TSA-CAfile msroot.pem "$EXE" >/dev/null 2>&1; then
  if [[ "$FORCE" -eq 0 ]]; then
    echo "$EXE in $TAG is already validly signed; use --force to re-sign."
    exit 0
  fi
  echo "==> Already signed, re-signing because of --force"
  osslsigncode remove-signature -in "$EXE" -out unsigned.exe >/dev/null
  mv unsigned.exe "$EXE"
fi

echo "==> Requesting Artifact Signing token from Azure CLI"
TOKEN=$(az account get-access-token --resource "https://codesigning.azure.net" --query accessToken -o tsv)

echo "==> Signing $EXE with $PROFILE"
jsign --storetype TRUSTEDSIGNING \
  --keystore "$ENDPOINT" \
  --storepass "$TOKEN" \
  --alias "$PROFILE" \
  --alg SHA-256 \
  --tsaurl http://timestamp.acs.microsoft.com \
  --tsmode RFC3161 \
  "$EXE"

echo "==> Verifying signature"
osslsigncode verify -CAfile msroot.pem -TSA-CAfile msroot.pem "$EXE" \
  | grep -E "Signature verification:|^Succeeded|^Failed" | sed 's/^/    /'

rm -f "$ASSET"
zip -q -j "$ASSET" "$EXE"

echo "==> Replacing $ASSET on release $TAG"
gh release upload "$TAG" --repo "$REPO" --clobber "$ASSET"
echo "Done: $TAG now ships a signed $EXE"
