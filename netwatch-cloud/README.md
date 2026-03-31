# Netwatch Cloud

Network monitoring cloud platform.

## Status

![Security Audit](https://github.com/your-org/netwatch-cloud/actions/workflows/security-audit.yml/badge.svg)

## Security

This project runs automated security audits via GitHub Actions CI/CD pipeline:

- **Cargo Audit**: Checks Rust crate dependencies for known vulnerabilities
- **NPM Audit**: Checks JavaScript/Node.js dependencies for vulnerabilities (if applicable)
- **Cargo Outdated**: Weekly informational report of outdated dependencies

Security audits run:
- On every push to `main`
- On every pull request
- Weekly on Sundays (automated schedule)

### Running Audits Locally

**Rust dependencies:**
```bash
cargo install cargo-audit
cargo audit --deny warnings
```

**Node.js dependencies (if applicable):**
```bash
npm audit --production
```

**Outdated dependencies (informational):**
```bash
cargo install cargo-outdated
cargo outdated --root-deps
```

## Getting Started

See [SPEC.md](SPEC.md) for full specification and [INDEX.md](INDEX.md) for project documentation.

## License

See LICENSE file for details.
