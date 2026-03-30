#!/bin/sh
set -e

# NetWatch Agent installer & updater
#
# Install:  curl -sSL <api-url>/install.sh | sudo sh -s -- --api-key KEY --endpoint URL
# Update:   curl -sSL <api-url>/install.sh | sudo sh -s -- --update
# Remove:   curl -sSL <api-url>/install.sh | sudo sh -s -- --remove

API_KEY=""
ENDPOINT=""
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/netwatch-agent"
DATA_DIR="/var/lib/netwatch-agent"
SERVICE_USER="netwatch"
MODE="install"

# Parse args
while [ $# -gt 0 ]; do
  case $1 in
    --api-key)   API_KEY="$2"; shift 2 ;;
    --endpoint)  ENDPOINT="$2"; shift 2 ;;
    --update)    MODE="update"; shift ;;
    --remove)    MODE="remove"; shift ;;
    --help|-h)   MODE="help"; shift ;;
    *)           echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ── Help ─────────────────────────────────────────────────
if [ "$MODE" = "help" ]; then
  echo "NetWatch Agent Installer"
  echo ""
  echo "INSTALL:"
  echo "  sudo ./install.sh --api-key YOUR_KEY --endpoint https://your-api/api/v1/ingest"
  echo ""
  echo "UPDATE (preserves config):"
  echo "  sudo ./install.sh --update"
  echo ""
  echo "REMOVE:"
  echo "  sudo ./install.sh --remove"
  exit 0
fi

# ── Remove ───────────────────────────────────────────────
if [ "$MODE" = "remove" ]; then
  echo "Removing NetWatch Agent..."
  systemctl stop netwatch-agent 2>/dev/null || true
  systemctl disable netwatch-agent 2>/dev/null || true
  rm -f /etc/systemd/system/netwatch-agent.service
  rm -f "$INSTALL_DIR/netwatch-agent"
  systemctl daemon-reload
  echo "✅ Agent removed. Config preserved at $CONFIG_DIR"
  echo "   To delete config: rm -rf $CONFIG_DIR $DATA_DIR"
  exit 0
fi

# ── Check root ───────────────────────────────────────────
if [ "$(id -u)" -ne 0 ]; then
  echo "Error: This script must be run as root (use sudo)"
  exit 1
fi

# ── Update mode: read existing config ────────────────────
if [ "$MODE" = "update" ]; then
  if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Error: No existing config found at $CONFIG_DIR/config.toml"
    echo "       Run with --api-key and --endpoint for a fresh install."
    exit 1
  fi
  OLD_VERSION=$("$INSTALL_DIR/netwatch-agent" --version 2>/dev/null || echo "unknown")
  echo "Updating NetWatch Agent (current: $OLD_VERSION)..."
fi

# ── Install mode: validate args ──────────────────────────
if [ "$MODE" = "install" ]; then
  if [ -z "$API_KEY" ]; then
    echo "Error: --api-key is required"
    echo "Usage: $0 --api-key YOUR_API_KEY --endpoint YOUR_ENDPOINT_URL"
    exit 1
  fi
  if [ -z "$ENDPOINT" ]; then
    echo "Error: --endpoint is required"
    echo "Usage: $0 --api-key YOUR_API_KEY --endpoint YOUR_ENDPOINT_URL"
    exit 1
  fi
fi

# ── Detect architecture ─────────────────────────────────
ARCH=$(uname -m)
case $ARCH in
  x86_64)  ARCH="x86_64" ;;
  aarch64) ARCH="aarch64" ;;
  arm64)   ARCH="aarch64" ;;
  *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "NetWatch Agent Installer"
echo "========================"
echo "Architecture: $ARCH"
echo "Mode: $MODE"
echo ""

# ── Download binary ──────────────────────────────────────
REPO="matthart1983/netwatch-cloud"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/netwatch-agent-linux-${ARCH}"
echo "Downloading agent from $DOWNLOAD_URL ..."

if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o /tmp/netwatch-agent "$DOWNLOAD_URL" 2>/dev/null
elif command -v wget >/dev/null 2>&1; then
  wget -qO /tmp/netwatch-agent "$DOWNLOAD_URL"
else
  echo "Error: curl or wget is required"
  exit 1
fi

if [ ! -f /tmp/netwatch-agent ] || [ ! -s /tmp/netwatch-agent ]; then
  echo ""
  echo "Error: Failed to download agent binary."
  echo ""
  echo "This can happen if:"
  echo "  - No release has been published yet (run: git tag v0.1.0 && git push origin v0.1.0)"
  echo "  - The repository is private"
  echo ""
  echo "Alternative: build from source"
  echo "  cargo build --release --package netwatch-agent"
  echo "  sudo cp target/release/netwatch-agent /usr/local/bin/"
  exit 1
fi

chmod +x /tmp/netwatch-agent
NEW_VERSION=$(/tmp/netwatch-agent --version 2>/dev/null || echo "unknown")

# ── Stop service if running ──────────────────────────────
systemctl stop netwatch-agent 2>/dev/null || true

# ── Create service user (install only) ───────────────────
if [ "$MODE" = "install" ]; then
  if ! id "$SERVICE_USER" >/dev/null 2>&1; then
    echo "Creating service user '$SERVICE_USER' ..."
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
  fi
fi

# ── Install binary ───────────────────────────────────────
echo "Installing binary ($NEW_VERSION) to $INSTALL_DIR/netwatch-agent ..."
mv /tmp/netwatch-agent "$INSTALL_DIR/netwatch-agent"

# ── Write config (install only — update preserves config) ─
if [ "$MODE" = "install" ]; then
  echo "Writing config to $CONFIG_DIR/config.toml ..."
  mkdir -p "$CONFIG_DIR"
  cat > "$CONFIG_DIR/config.toml" <<EOF
# NetWatch Agent configuration
endpoint = "$ENDPOINT"
api_key = "$API_KEY"
interval_secs = 15
health_interval_secs = 30
EOF
  chmod 600 "$CONFIG_DIR/config.toml"
  chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_DIR/config.toml"

  # Create data directory
  mkdir -p "$DATA_DIR"
  chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
fi

# ── Install systemd unit ─────────────────────────────────
cat > /etc/systemd/system/netwatch-agent.service <<EOF
[Unit]
Description=NetWatch Agent — network metrics collector
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/netwatch-agent
Restart=always
RestartSec=5
User=$SERVICE_USER
Group=$SERVICE_USER
EnvironmentFile=-$CONFIG_DIR/env

NoNewPrivileges=no
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$DATA_DIR
ReadOnlyPaths=$CONFIG_DIR /proc /sys
PrivateTmp=yes
AmbientCapabilities=CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

# ── Enable and start ─────────────────────────────────────
systemctl daemon-reload
systemctl enable netwatch-agent
systemctl start netwatch-agent

echo ""
if [ "$MODE" = "update" ]; then
  echo "✅ NetWatch Agent updated! ($OLD_VERSION → $NEW_VERSION)"
else
  echo "✅ NetWatch Agent installed and running! ($NEW_VERSION)"
fi
echo ""
echo "  Status:  systemctl status netwatch-agent"
echo "  Logs:    journalctl -u netwatch-agent -f"
echo "  Config:  $CONFIG_DIR/config.toml"
echo "  Update:  curl -sSL <api-url>/install.sh | sudo sh -s -- --update"
echo "  Remove:  curl -sSL <api-url>/install.sh | sudo sh -s -- --remove"
