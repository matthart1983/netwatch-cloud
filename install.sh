#!/bin/bash
set -e

# NetWatch Agent installer
# Usage: curl -sSL https://install.netwatch.dev | sh -s -- --api-key YOUR_KEY
#    or: ./install.sh --api-key YOUR_KEY --endpoint https://api.netwatch.dev/api/v1/ingest

API_KEY=""
ENDPOINT="https://api.netwatch.dev/api/v1/ingest"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/netwatch-agent"
DATA_DIR="/var/lib/netwatch-agent"
SERVICE_USER="netwatch"

# Parse args
while [[ $# -gt 0 ]]; do
  case $1 in
    --api-key)   API_KEY="$2"; shift 2 ;;
    --endpoint)  ENDPOINT="$2"; shift 2 ;;
    *)           echo "Unknown option: $1"; exit 1 ;;
  esac
done

if [ -z "$API_KEY" ]; then
  echo "Error: --api-key is required"
  echo "Usage: $0 --api-key YOUR_API_KEY [--endpoint URL]"
  exit 1
fi

# Detect architecture
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
echo "Endpoint:     $ENDPOINT"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo "Error: This script must be run as root (use sudo)"
  exit 1
fi

# Download binary
REPO="matthart1983/netwatch-cloud"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/netwatch-agent-linux-${ARCH}"
echo "Downloading agent from $DOWNLOAD_URL ..."

if command -v curl &> /dev/null; then
  HTTP_CODE=$(curl -fsSL -o /tmp/netwatch-agent -w "%{http_code}" "$DOWNLOAD_URL" 2>/dev/null)
elif command -v wget &> /dev/null; then
  wget -qO /tmp/netwatch-agent "$DOWNLOAD_URL"
  HTTP_CODE=$?
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
  echo "Alternative install methods:"
  echo "  1. Docker:  docker run -d -e NETWATCH_API_KEY=YOUR_KEY -e NETWATCH_ENDPOINT=YOUR_URL netwatch-agent"
  echo "  2. Source:   cargo build --release --package netwatch-agent && sudo cp target/release/netwatch-agent /usr/local/bin/"
  exit 1
fi

chmod +x /tmp/netwatch-agent

# Create service user (no login shell, no home dir)
if ! id "$SERVICE_USER" &>/dev/null; then
  echo "Creating service user '$SERVICE_USER' ..."
  useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi

# Install binary
echo "Installing binary to $INSTALL_DIR/netwatch-agent ..."
mv /tmp/netwatch-agent "$INSTALL_DIR/netwatch-agent"

# Create config
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

# Create data directory (for host-id persistence)
mkdir -p "$DATA_DIR"
chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"

# Install systemd unit
echo "Installing systemd service ..."
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

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$DATA_DIR
ReadOnlyPaths=$CONFIG_DIR
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
systemctl daemon-reload
systemctl enable netwatch-agent
systemctl start netwatch-agent

echo ""
echo "✅ NetWatch Agent installed and running!"
echo ""
echo "  Status:  systemctl status netwatch-agent"
echo "  Logs:    journalctl -u netwatch-agent -f"
echo "  Config:  $CONFIG_DIR/config.toml"
echo "  Stop:    systemctl stop netwatch-agent"
echo "  Remove:  systemctl stop netwatch-agent && systemctl disable netwatch-agent && rm $INSTALL_DIR/netwatch-agent"
