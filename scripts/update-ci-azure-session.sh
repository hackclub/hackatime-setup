#!/usr/bin/env bash
# Log in to Azure with your own account and store that Azure CLI session as the
# AZURE_CLI_SESSION secret in the release-signing GitHub environment, so the
# Windows release job can sign hackatime_cli.exe as you.
#
# Only needed while the hackatime-setup-github service principal lacks the
# "Artifact Signing Certificate Profile Signer" role. Re-run it when CI reports
# that the stored session no longer works (Entra refresh tokens expire after
# roughly 90 days of inactivity or on password/MFA changes).
#
# Usage: scripts/update-ci-azure-session.sh [--from-dir <AZURE_CONFIG_DIR>]
set -euo pipefail

REPO="hackclub/hackatime-setup"
ENVIRONMENT="release-signing"
SECRET="AZURE_CLI_SESSION"

for tool in az gh tar base64; do
  command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 1; }
done

DIR=""
if [[ "${1:-}" == "--from-dir" ]]; then
  DIR="${2:?--from-dir needs a path}"
fi

if [[ -z "$DIR" ]]; then
  DIR=$(mktemp -d)
  trap 'rm -rf "$DIR"' EXIT
  # A clean config dir so the exported session contains only this login.
  printf '[core]\nencrypt_token_cache = false\ncollect_telemetry = false\n' > "$DIR/config"
  echo "==> Logging in (a browser window will open)"
  AZURE_CONFIG_DIR="$DIR" az login --allow-no-subscriptions --query "[].user.name" -o tsv
fi

echo "==> Checking the session can mint an Artifact Signing token"
AZURE_CONFIG_DIR="$DIR" az account get-access-token \
  --resource https://codesigning.azure.net --query expiresOn -o tsv

echo "==> Uploading session to $REPO environment '$ENVIRONMENT' as $SECRET"
tar -C "$DIR" -czf - config azureProfile.json msal_token_cache.json \
  | base64 \
  | gh secret set "$SECRET" --repo "$REPO" --env "$ENVIRONMENT"
echo "Done"
