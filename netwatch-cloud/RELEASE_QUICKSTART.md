# Release Quick Start

## Create a Release in 2 Commands

```bash
# 1. Create and push version tag
git tag v0.1.0 && git push --tags

# 2. Wait ~5 minutes for workflow to complete
# (Watch: GitHub Actions tab in repository)
```

## Get Your Binaries

1. Go to: `https://github.com/YOUR_ORG/netwatch-cloud/releases`
2. Download your platform's binary
3. Verify checksum: `sha256sum -c checksums.txt`

## What You're Getting

✅ 5 pre-compiled binaries (one per platform):
- `netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu`
- `netwatch-cloud-v0.1.0-aarch64-unknown-linux-gnu`
- `netwatch-cloud-v0.1.0-x86_64-apple-darwin`
- `netwatch-cloud-v0.1.0-aarch64-apple-darwin`
- `netwatch-cloud-v0.1.0-x86_64-pc-windows-msvc.exe`

✅ `checksums.txt` for verification

✅ Auto-generated release notes

## Verify Binary

```bash
# Download checksums
wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/checksums.txt

# Verify binary
sha256sum -c checksums.txt
```

## Install Binary

```bash
# Linux example
chmod +x netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu
sudo mv netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu /usr/local/bin/netwatch-cloud
netwatch-cloud --version
```

## What Happens Automatically

✅ GitHub Actions builds 5 platforms in parallel (4-5 min total)
✅ Binaries are stripped for smaller size
✅ SHA256 checksums computed for all binaries
✅ Release created with all artifacts
✅ Pre-release flag set for v0.x versions
✅ Release notes auto-generated

## Full Documentation

See: [RELEASE_WORKFLOW.md](./RELEASE_WORKFLOW.md)

## Tag Format

- ✅ **v0.1.0** (matches pattern)
- ✅ **v1.0.0** (matches pattern)
- ❌ **0.1.0** (missing 'v')
- ❌ **release-0.1.0** (wrong pattern)

## Common Issues

**"Workflow didn't start"**
- Push tag with `git push --tags`
- Check GitHub Actions tab is enabled

**"Build failed for one platform"**
- Check logs: GitHub Actions > Workflow > specific platform
- Cross-compilation issues? Check [RELEASE_WORKFLOW.md](./RELEASE_WORKFLOW.md#troubleshooting)

**"Checksum mismatch"**
- Download both file and checksums.txt from same release
- Use exact binary name in checksum file

---

Ready? Go release: `git tag v0.1.0 && git push --tags` 🚀
