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
secret or a checked-in certificate. An interactive `az login` is only needed
when signing manually from a developer machine.
