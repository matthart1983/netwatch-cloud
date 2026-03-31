# GitHub Actions Release Workflow

Cross-platform binary builds and automated GitHub releases for netwatch-cloud.

## Features

✅ **Automatic Cross-Platform Builds**
- Linux x86_64 (GNU)
- Linux aarch64/ARM64 (GNU)
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64 (MSVC)

✅ **Binary Optimization**
- Release build (`--release`)
- Binary stripping (size reduction)
- SHA256 checksum generation

✅ **Automated GitHub Release**
- Triggered on git tag push (v*.*.*)
- All binaries uploaded as artifacts
- Combined `checksums.txt` for verification
- Auto-generated release notes
- Pre-release marking for v0.x versions

✅ **Build Caching**
- Cargo registry cache
- Cargo git cache
- Target build cache

## Usage

### Create a Release

```bash
# Tag a release (e.g., v0.1.0)
git tag v0.1.0

# Push tag (triggers workflow)
git push --tags

# Or push a specific tag
git push origin v0.1.0
```

The GitHub Actions workflow will:
1. Build binaries for all 5 platforms in parallel
2. Strip binaries to reduce size
3. Generate SHA256 checksums
4. Create a GitHub Release with all artifacts
5. Generate release notes

### Download Binaries

Visit: `https://github.com/YOUR_ORG/netwatch-cloud/releases`

Example:
```bash
# Linux x86_64
wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu

# macOS Apple Silicon
wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/netwatch-cloud-v0.1.0-aarch64-apple-darwin

# Windows
wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/netwatch-cloud-v0.1.0-x86_64-pc-windows-msvc.exe
```

### Verify Binaries

```bash
# Download checksums
wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/checksums.txt

# Verify all binaries
sha256sum -c checksums.txt

# Verify single binary
sha256sum -c <(grep "netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu" checksums.txt)
```

## Workflow File

**Location:** `.github/workflows/release.yml`

### Matrix Configuration

| Platform | OS | Target | Cross |
|----------|----|----|---------|
| Linux x86_64 | ubuntu-latest | x86_64-unknown-linux-gnu | No |
| Linux ARM64 | ubuntu-latest | aarch64-unknown-linux-gnu | Yes* |
| macOS Intel | macos-latest | x86_64-apple-darwin | No |
| macOS Apple Silicon | macos-latest | aarch64-apple-darwin | No |
| Windows x86_64 | windows-latest | x86_64-pc-windows-msvc | No |

*Uses `cross` tool for ARM64 cross-compilation on x86_64 Linux

### Build Steps

1. **Checkout** - Fetch full repository history
2. **Install Rust** - Uses dtolnay/rust-toolchain@stable
3. **Install cross** - Only for ARM64 Linux
4. **Cache Setup** - Registry, git, and build targets
5. **Build** - `cargo build --release --target <target> --package netwatch-cloud`
6. **Strip** - Reduce binary size (Unix only)
7. **Version Extraction** - Extract version from git tag
8. **Binary Naming** - Create versioned filename
9. **Checksum** - Generate SHA256 hash
10. **Upload Artifacts** - Temporary storage for release job
11. **Create Release** - Upload to GitHub with checksums

### Job Dependencies

- **build** (5 parallel matrix jobs) - Compiles binaries
- **create-release** (depends on build) - Creates GitHub release

## Customization

### Change Package Name

Update in `.github/workflows/release.yml`:
```yaml
cargo build --release --target ${{ matrix.target }} --package YOUR_PACKAGE_NAME
```

### Add More Platforms

Add to the matrix in `build` job:
```yaml
- os: ubuntu-latest
  target: riscv64gc-unknown-linux-gnu
  binary_name: netwatch-cloud
  use_cross: true
```

### Change Tag Pattern

Update the trigger in `on.push.tags`:
```yaml
- 'release-[0-9]+.[0-9]+.[0-9]+'
```

### Modify Release Notes

Edit the `body:` section in `create-release` step for custom release notes.

## Troubleshooting

### ARM64 Build Fails

The workflow uses `cross` tool for ARM64 Linux builds:
```bash
# Manual ARM64 build (if needed):
cargo install cross --locked
cross build --release --target aarch64-unknown-linux-gnu
```

### Binary Not Stripping

Windows binaries are excluded from stripping (MSVC doesn't have `strip`).

### Checksum Verification Fails

1. Ensure you downloaded the correct binary for your platform
2. Check line endings: `checksums.txt` must use Unix line endings (LF)
3. Verify file is complete: `wc -c netwatch-cloud-v0.1.0-*`

## Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGES.md` with release notes
- [ ] Create annotated tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
- [ ] Push tag: `git push --tags`
- [ ] Monitor GitHub Actions workflow (watch status badge)
- [ ] Verify binaries in GitHub Release page
- [ ] Test at least one platform binary locally

## Security

- Binaries are built from source on GitHub Actions runners
- SHA256 checksums provided for verification
- No external binary uploads or storage
- GitHub GITHUB_TOKEN automatically provided

## Performance

**Typical Build Times:**
- Linux x86_64: ~2-3 minutes
- Linux ARM64: ~3-4 minutes (with cross)
- macOS x86_64: ~2-3 minutes
- macOS Apple Silicon: ~2-3 minutes
- Windows: ~3-4 minutes

**Total parallel time:** ~4-5 minutes (jobs run in parallel)

## Notes

- Pre-release flag automatically set for `v0.x.x` versions
- Release artifact retention: 1 day (clean up automatically)
- Cargo cache: 7 days default (GitHub Actions)
- Full git history fetched for changelog generation
