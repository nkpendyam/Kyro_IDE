# Kyro IDE Installer Behavior

This document describes installer behavior and expected user experience for packaged Kyro builds.

## Windows (NSIS + MSI)

Kyro ships:
- NSIS `.exe` installer (primary user flow)
- MSI package (enterprise deployment scenarios)

Windows hooks are defined in `src-tauri/windows/installer-hooks.nsh`.

### Install behavior

Post-install hooks configure:
- Desktop shortcut creation
- Start Menu entry under `Kyro IDE`
- Shell context menu entries for files and folders (`Open with Kyro IDE`)
- `kyro` command launcher in `%LOCALAPPDATA%\Microsoft\WindowsApps\kyro.cmd`

### Uninstall behavior

Post-uninstall hooks remove:
- `kyro` launcher command
- File/folder context menu registry keys

## macOS (DMG)

Kyro builds a DMG installer bundle.

Release workflow supports optional notarization when these secrets are configured:
- `APPLE_NOTARY_APPLE_ID`
- `APPLE_NOTARY_APP_PASSWORD`
- `APPLE_NOTARY_TEAM_ID`

## Linux (AppImage/DEB)

Kyro build targets include AppImage and DEB packaging based on host toolchain availability.

## Optional Signing and Notarization

Release workflow supports optional signing paths:

### Windows signing
- `WINDOWS_CERTIFICATE_BASE64` (base64-encoded PFX)
- `WINDOWS_CERTIFICATE_PASSWORD`

When set, Windows MSI/NSIS artifacts are signed via `signtool`.

### macOS code-signing import
- `APPLE_CERTIFICATE_BASE64` (base64-encoded P12)
- `APPLE_CERTIFICATE_PASSWORD`

If these are not set, builds still run unsigned for internal/dev usage.

## Integrity Smoke Checks

Release workflow computes SHA-256 checksums for generated artifacts and uploads them as separate `*-checksums` artifacts.
