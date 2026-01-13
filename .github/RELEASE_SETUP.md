# Release CI/CD Setup Guide

This document describes how to set up the automated release pipeline for `rgb-lib` and its Go bindings.

## Overview

The release workflow:

1. **Triggered by**: Creating a new git tag (`v*`) or manual workflow dispatch
2. **Builds**: Native libraries for supported platforms (Linux x64, macOS ARM64)
3. **Creates**: GitHub Release with all artifacts
4. **Triggers**: Automatic rebuild of `rgb-lib-go` bindings

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         rgb-lib Repository                        │
│                                                                   │
│  push tag v* ──► release.yml                                     │
│                     │                                             │
│                     ├── build-uniffi (2 targets)                 │
│                     ├── build-cffi (2 targets)                   │
│                     ├── generate-header                          │
│                     │                                             │
│                     ▼                                             │
│              Create GitHub Release                                │
│                     │                                             │
│                     ▼                                             │
│          repository_dispatch ─────────────────────────────────────┤
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│                UTEXO-Protocol/rgb-lib-go Repository               │
│                                                                   │
│  repository_dispatch ──► release.yml                              │
│        (rgb-lib-release)     │                                    │
│                              ├── Download rgb-lib artifacts       │
│                              ├── Generate Go bindings (uniffi)    │
│                              ├── Build for all platforms          │
│                              │                                    │
│                              ▼                                    │
│                       Create GitHub Release                       │
└───────────────────────────────────────────────────────────────────┘
```

## Setup Instructions

### Step 1: Create Personal Access Token (PAT)

To trigger workflows in the `rgb-lib-go` repository, you need a PAT with cross-repo permissions.

1. Go to GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens
2. Click "Generate new token"
3. Configure:
   - **Token name**: `rgb-lib-release-trigger`
   - **Expiration**: Set according to your security policy (recommended: 1 year)
   - **Repository access**: "Only select repositories" → Select `UTEXO-Protocol/rgb-lib-go`
   - **Permissions**:
     - Contents: Read and write
     - Actions: Read and write (required for repository_dispatch)
4. Click "Generate token" and copy the token value

### Step 2: Add Secret to rgb-lib Repository

1. Go to `UTEXO-Protocol/rgb-lib` → Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Configure:
   - **Name**: `RGB_LIB_GO_PAT`
   - **Secret**: Paste the PAT from Step 1
4. Click "Add secret"

### Step 3: rgb-lib-go Repository

The workflow is already set up in [UTEXO-Protocol/rgb-lib-go](https://github.com/UTEXO-Protocol/rgb-lib-go) at `.github/workflows/release.yml`.

It will automatically trigger when rgb-lib publishes a new release.

## Usage

### Creating a Release

#### Option 1: Git Tag (Recommended)

```bash
# Create and push a tag
git tag v0.3.0-beta.5
git push origin v0.3.0-beta.5
```

This will automatically:
1. Build all artifacts
2. Create a GitHub Release
3. Trigger the Go bindings rebuild

#### Option 2: Manual Dispatch

1. Go to Actions → Release workflow
2. Click "Run workflow"
3. Enter the version (e.g., `v0.3.0-beta.5`)
4. Click "Run workflow"

### Release Assets

Each release includes:

| Asset | Description |
|-------|-------------|
| `rgb-lib-uniffi-{target}.zip` | UniFFI shared library for the target platform |
| `rgb-lib-cffi-{target}.zip` | C-FFI static/shared libraries + header file |

Supported targets:
- `x86_64-unknown-linux-gnu` - Linux x64
- `aarch64-apple-darwin` - macOS ARM64 (Apple Silicon)

## Troubleshooting

### PAT Token Expired

If the Go bindings trigger fails with authentication errors:
1. Generate a new PAT (Step 1)
2. Update the `RGB_LIB_GO_PAT` secret (Step 2)

### Build Failures

Check the build logs in GitHub Actions for specific errors. Common issues:
- Dependency resolution issues (check Cargo.lock)
- Rust toolchain version mismatch

### Go Bindings Not Triggering

Verify:
1. The PAT has correct permissions
2. The secret name matches (`RGB_LIB_GO_PAT`)
3. The target repository path is correct (`UTEXO-Protocol/rgb-lib-go`)

## Security Considerations

1. **PAT Scope**: The PAT only needs access to the specific repository, not your entire GitHub account
2. **Secret Rotation**: Rotate the PAT periodically (at least annually)
3. **Audit Logs**: Monitor GitHub Actions audit logs for unauthorized workflow runs

## Customization

### Adding More Targets

To add additional build targets, update the matrix in `release.yml`:

```yaml
matrix:
  include:
    - target: new-target-triple
      os: runner-os
      lib_name: library-name
```

### Changing Trigger Conditions

Modify the `on` section in `release.yml`:

```yaml
on:
  push:
    tags:
      - 'v*'      # All version tags
      - 'release-*'  # Custom prefix
```

### Different Downstream Repositories

To trigger multiple downstream repositories, add more dispatch steps:

```yaml
- name: Trigger another-repo
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ secrets.ANOTHER_REPO_PAT }}
    repository: org/another-repo
    event-type: rgb-lib-release
```

