# Release Checklist

Complete this checklist before creating a release.

## Pre-Release (✓ One Day Before)

- [ ] Update `Cargo.toml` version number
  ```toml
  [package]
  version = "0.1.0"  # ← Update this
  ```

- [ ] Update `CHANGES.md` with release notes
  ```markdown
  ## v0.1.0 - 2026-03-31
  
  ### Features
  - Feature 1
  - Feature 2
  
  ### Bug Fixes
  - Fix 1
  ```

- [ ] Verify build passes locally
  ```bash
  cargo build --release
  cargo test
  cargo clippy
  ```

- [ ] Commit all changes
  ```bash
  git add .
  git commit -m "chore: prepare release v0.1.0"
  git push
  ```

## Create Release (✓ Release Day)

- [ ] Create annotated tag
  ```bash
  git tag -a v0.1.0 -m "Release v0.1.0"
  ```

- [ ] Push tag to GitHub
  ```bash
  git push origin v0.1.0
  # or
  git push --tags
  ```

- [ ] Monitor GitHub Actions
  - Go to: `https://github.com/YOUR_ORG/netwatch-cloud/actions`
  - Watch "Release" workflow
  - Wait for all 5 platform builds to complete
  - Expected time: 4-5 minutes

## Post-Release (✓ After Build Completes)

- [ ] Verify GitHub Release created
  - Go to: `https://github.com/YOUR_ORG/netwatch-cloud/releases`
  - Check release appears
  - Check all 5 binaries uploaded

- [ ] Verify checksums.txt exists
  ```bash
  # Download
  wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/checksums.txt
  
  # Inspect
  cat checksums.txt
  ```

- [ ] Test download & verify (at least one platform)
  ```bash
  # Linux x86_64 example
  wget https://github.com/YOUR_ORG/netwatch-cloud/releases/download/v0.1.0/netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu
  chmod +x netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu
  
  # Verify checksum
  sha256sum netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu
  # Match against checksums.txt
  ```

- [ ] Test binary execution
  ```bash
  ./netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu --version
  ```

- [ ] Test on another platform (optional)
  ```bash
  # macOS example
  # Download, verify, and test on macOS
  ```

## Announce Release (✓ Optional)

- [ ] Update project README with new version
- [ ] Post release announcement on discussion board
- [ ] Update website/docs with new version
- [ ] Tag release in release notes format

## Troubleshooting

### Workflow Didn't Start
- [ ] Verify tag matches pattern: `v0.1.0`, `v1.2.3`, etc.
- [ ] Check tag was pushed: `git push --tags`
- [ ] Verify GitHub Actions enabled in repository settings
- [ ] Check workflow file syntax: `.github/workflows/release.yml`

### Build Failed on Platform X
- [ ] Check GitHub Actions logs for specific error
- [ ] Common issues:
  - ARM64 cross-compilation: needs `cross` tool
  - macOS: might need xcode-select
  - Windows: MSVC toolchain required
- [ ] Manual build test:
  ```bash
  # For the failing platform
  cargo build --release --target <target>
  ```

### Checksum Mismatch
- [ ] Ensure downloaded file is complete
- [ ] Use exact filename from checksums.txt
- [ ] Verify line endings (LF, not CRLF)
- [ ] Re-download both file and checksums.txt

### Binaries Not in Release
- [ ] Check artifact upload step in workflow logs
- [ ] Verify `create-release` job ran successfully
- [ ] Check release creation didn't fail

## Tag Format Reference

✅ **Valid tags:**
- v0.1.0
- v1.2.3
- v10.20.30
- v1.0.0-beta (if pattern updated)

❌ **Invalid tags:**
- 0.1.0 (missing 'v')
- v0.1 (incomplete version)
- release-0.1.0 (wrong prefix)
- v0.1.0-rc1 (needs pattern update)

## Binary Distribution

After release, you can use these patterns:

### Install Script
```bash
#!/bin/bash
VERSION="v0.1.0"
PLATFORM="x86_64-unknown-linux-gnu"
BINARY="netwatch-cloud-$VERSION-$PLATFORM"
URL="https://github.com/YOUR_ORG/netwatch-cloud/releases/download/$VERSION/$BINARY"

wget "$URL"
chmod +x "$BINARY"
sudo mv "$BINARY" /usr/local/bin/netwatch-cloud
```

### Homebrew (optional)
Create formula in homebrew-netwatch-cloud repo

### Docker (optional)
```dockerfile
FROM debian:bookworm-slim
COPY netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu /usr/local/bin/netwatch-cloud
RUN chmod +x /usr/local/bin/netwatch-cloud
ENTRYPOINT ["netwatch-cloud"]
```

## Version Management

**Semantic Versioning:**
- `MAJOR.MINOR.PATCH`
- Increment MAJOR: Breaking changes
- Increment MINOR: New features
- Increment PATCH: Bug fixes

**Examples:**
- v0.1.0 → v0.2.0 (new feature)
- v0.1.0 → v0.1.1 (bug fix)
- v0.1.0 → v1.0.0 (first stable)

## Documentation

- **Full Guide:** [RELEASE_WORKFLOW.md](./RELEASE_WORKFLOW.md)
- **Quick Start:** [RELEASE_QUICKSTART.md](./RELEASE_QUICKSTART.md)
- **Workflow File:** [.github/workflows/release.yml](./.github/workflows/release.yml)

---

## Quick Commands Reference

```bash
# View release workflow
cat .github/workflows/release.yml

# Create release
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Watch workflow
open https://github.com/YOUR_ORG/netwatch-cloud/actions

# View releases
open https://github.com/YOUR_ORG/netwatch-cloud/releases

# Verify binary
sha256sum -c <(grep "netwatch-cloud-v0.1.0-x86_64-unknown-linux-gnu" checksums.txt)
```

---

**Total Release Time:** ~15 minutes
- Preparation: 5 minutes
- GitHub Actions build: 4-5 minutes
- Verification: 5 minutes
