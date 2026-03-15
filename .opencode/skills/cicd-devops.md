---
name: cicd-devops
description: Use this skill for all CI/CD, GitHub Actions, deployment, versioning, release automation, Docker, and DevOps tasks. Triggers on: "workflow", "CI", "deploy", "release", "docker", "action", "pipeline", "version", "tag".
---

# CI/CD and DevOps Skill

## Kyro IDE release pipeline overview

```
You push: git tag v1.0.0
    ↓
GitHub Actions triggers release.yml
    ↓
Tests run on Windows + macOS + Linux
    ↓
Builds: MSI, DMG x2, AppImage, .deb
    ↓
Creates GitHub Release with all installers
    ↓
Uploads latest.json for auto-updater
    ↓
Existing users get update popup
```

## Version bump workflow (run before every release)

```bash
# 1. Decide version (semver: MAJOR.MINOR.PATCH)
# Fix only → PATCH: v1.0.0 → v1.0.1
# New feature → MINOR: v1.0.0 → v1.1.0
# Breaking change → MAJOR: v1.0.0 → v2.0.0

# 2. Update version in all 3 places
# package.json → "version": "X.Y.Z"
# src-tauri/Cargo.toml → version = "X.Y.Z"
# src-tauri/tauri.conf.json → "version": "X.Y.Z"

# 3. Generate changelog
git cliff --unreleased -o CHANGELOG.md

# 4. Commit
git add .
git commit -m "chore: bump version to vX.Y.Z"

# 5. Tag and push
git tag vX.Y.Z -m "Kyro IDE vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

## Checking CI before releasing

```bash
# Check all workflows are green
gh run list --branch main --limit 5

# View specific workflow
gh workflow view ci.yml

# If something failed, see why
gh run view --log-failed
```

## Deployment environments

### Beta (pre-release)
```bash
git tag v0.9.0-beta
git push origin v0.9.0-beta
# GitHub marks as "Pre-release"
# Auto-updater only sends to beta channel users
```

### Stable (production)
```bash
git tag v1.0.0
git push origin v1.0.0
# GitHub marks as "Latest release"
# Auto-updater sends to ALL users
```

## Auto-updater setup (one-time)

```bash
# Generate signing keys
pnpm tauri signer generate -w ~/.tauri/kyro-ide.key
# Saves: ~/.tauri/kyro-ide.key (private) and .pub (public)

# Add to GitHub secrets
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/kyro-ide.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# Enter the password you chose when generating

# Put public key in tauri.conf.json
# "updater": { "pubkey": "PASTE_PUBLIC_KEY_HERE" }
```

## Monitoring releases

```bash
# Check release was created
gh release list

# Download a release to test
gh release download v1.0.0

# Delete a bad release
gh release delete v1.0.0-bad --yes
git tag -d v1.0.0-bad
git push origin :v1.0.0-bad
```

## Security in CI

All workflows must run:
```yaml
- run: cargo audit        # Rust dependency CVEs
- run: pnpm audit --audit-level=high  # JS dependency CVEs
- run: cargo clippy --workspace -- -D warnings  # Code quality
```

These are already in security.yml — run on every PR and weekly.
