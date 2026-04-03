# macOS Dev Agent Notes

This document is for local development and debugging only.

`netwatch-agent` is publicly supported as a Linux service install. On macOS, the agent can still be useful for private development, API testing, and dashboard verification, but it is not part of the public install path.

## What Works On macOS

- building `netwatch-agent` locally with Cargo
- collecting a subset of host, interface, disk, connection, and health data via the macOS implementations in `netwatch-core`
- sending snapshots to the production or local cloud API
- optional per-user background execution via `launchd`

## What Is Not Publicly Supported

- the `/install.sh` Linux service installer
- Homebrew or packaged macOS distribution for the cloud agent
- any guarantee that metrics match Linux exactly
- the Linux-oriented self-update flow

## Quick Start

Build the agent:

```bash
cargo build --package netwatch-agent
```

Configure it interactively:

```bash
./target/debug/netwatch-agent setup
```

Run it in the foreground:

```bash
RUST_LOG=info ./target/debug/netwatch-agent
```

## macOS Background Mode

Install a per-user `launchd` agent:

```bash
./target/debug/netwatch-agent launchd-install
```

Check status:

```bash
./target/debug/netwatch-agent status
```

Remove the `launchd` agent:

```bash
./target/debug/netwatch-agent launchd-remove
```

## Files Used On macOS

Config:

```text
~/.config/netwatch-agent/config.toml
```

Host ID:

```text
~/.config/netwatch-agent/host-id
```

Launchd plist:

```text
~/Library/LaunchAgents/com.netwatch.agent.dev.plist
```

## Resetting The Host Identity

If you want the Mac to appear as a brand-new host in the cloud UI, delete the persisted host ID and restart the agent:

```bash
rm ~/.config/netwatch-agent/host-id
RUST_LOG=info ./target/debug/netwatch-agent
```

## Notes

- The host will often appear as `Mac.localdomain` unless you override the hostname.
- If snapshots are accepted by the API but you do not see the host in the dashboard, double-check that the API key belongs to the same NetWatch Cloud account you are currently logged into.
