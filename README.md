# Hackatime Installer

A simple installer for Hackatime!

## Running the installer

Head to https://hackatime.hackclub.com/my/wakatime_setup to set it up!

## Supported tools
- VSCode, Cursor, Trae, Windsurf, Antigravity
- IntelliJ IDEs
- Zed
- Xcode
- Terminal (bash, zsh, fish)

## Windows release signing

Tagged releases sign `hackatime_cli.exe` with Azure Artifact Signing before it is
added to the Windows zip. The workflow uses the `hackclub` signing account and
the `hackatime-desktop` certificate profile at
`https://eus.codesigning.azure.net/`.

The GitHub repository must define these Actions secrets:

- `AZURE_CLIENT_ID`
- `AZURE_TENANT_ID`
- `AZURE_SUBSCRIPTION_ID`

They should identify a Microsoft Entra application configured with:

1. A federated GitHub credential for the `release-signing` environment, with
   subject `repo:hackclub/hackatime-setup:environment:release-signing`.
2. The **Artifact Signing Certificate Profile Signer** role on the signing
   account.

The workflow authenticates using GitHub OIDC, so it does not require a client
secret or a checked-in certificate.

If the service principal does not (yet) hold the signer role, the workflow
falls back to a stored Azure CLI session from the `AZURE_CLI_SESSION` secret in
the `release-signing` environment. Anyone whose own Azure account has the signer
role can create or rotate that secret (it expires after roughly 90 days of
inactivity or on credential changes):

```sh
brew install azure-cli
scripts/update-ci-azure-session.sh
```

If both fail, the release still ships with an unsigned `hackatime_cli.exe` and
the workflow prints a warning. The asset can then be signed and replaced from a
Mac or Linux machine:

```sh
brew install azure-cli jsign osslsigncode
az login
scripts/sign-windows-release.sh            # latest release
scripts/sign-windows-release.sh v1.5.24    # a specific tag
```

The script downloads the Windows zip, signs the exe with Azure Artifact Signing
via [jsign](https://ebourg.github.io/jsign/), verifies the signature against
Microsoft's root CA, and re-uploads the zip to the release.
