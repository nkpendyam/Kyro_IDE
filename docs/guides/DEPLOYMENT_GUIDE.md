# Kyro IDE Deployment Guide

This guide covers production deployment for Kyro desktop releases (Tauri bundles + updater artifacts).

## Release Model

Kyro releases are GitHub tag-driven:
- Stable: `vX.Y.Z`
- Beta: `vX.Y.Z-beta`
- Alpha: `vX.Y.Z-alpha`

`release.yml` automatically maps tags to channels:
- `stable` for plain semantic tags
- `beta` and `alpha` for prerelease tags

## Pre-Release Checklist

Before tagging:

1. Ensure `VERSION` matches release intent.
2. Ensure `CHANGELOG.md` contains `## [X.Y.Z]` for the tag.
3. Run local checks:
   - `bun run doctor`
   - `bun run lint`
   - `bun run type-check`
   - `bun run test`
   - `bun run build`
   - `cd src-tauri && cargo check --workspace --locked && cargo test --workspace --lib --tests`
4. Verify package output via `bun run tauri:build`.

## CI Release Pipeline

Workflow: `.github/workflows/release.yml`

For each target OS, the workflow:
1. Installs platform prerequisites.
2. Builds frontend static export (`out/`).
3. Runs Tauri release build.
4. Performs artifact smoke checks.
5. Computes SHA-256 checksums.
6. Uploads artifacts and checksum bundles.

Artifacts produced:
- Windows: NSIS `.exe` + MSI `.msi`
- macOS: `.dmg`
- Linux: `.AppImage`

## Optional Signing and Notarization

The release workflow supports optional trusted-distribution steps via secrets.

### Windows signing
- `WINDOWS_CERTIFICATE_BASE64` (PFX in base64)
- `WINDOWS_CERTIFICATE_PASSWORD`

When set, MSI and NSIS artifacts are signed with timestamping.

### macOS signing/notarization
- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_NOTARY_APPLE_ID`
- `APPLE_NOTARY_APP_PASSWORD`
- `APPLE_NOTARY_TEAM_ID`

When these are present, DMGs are notarized and stapled.

## Updater Strategy

Kyro uses `tauri-plugin-updater` and GitHub Releases as the update source.

Production expectations:
- Release tags are immutable once published.
- Artifacts remain attached to the matching release.
- Checksum artifacts are retained for audit/verification.
- Stable clients track stable tags only; prerelease clients may opt into beta/alpha channels.

## Post-Release Validation

After each release:
1. Confirm GitHub release draft contains all platform artifacts.
2. Download one artifact per platform and verify checksum.
3. Launch installed app and verify startup.
4. Confirm updater metadata resolves for the release channel.

## Rollback Guidance

If a bad release is detected:
1. Do not replace existing release assets in place.
2. Publish a newer patch tag (`vX.Y.(Z+1)`) with fixes.
3. Update `CHANGELOG.md` with remediation notes.
