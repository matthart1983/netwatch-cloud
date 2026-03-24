#!/bin/bash
set -e

# NetWatch Agent Manager
# Usage: ./agent.sh [start|stop|update|logs|status|config]

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENV_FILE="$SCRIPT_DIR/.agent.env"
IMAGE_NAME="netwatch-agent"
CONTAINER_NAME="netwatch-agent"

# ── Load saved config ────────────────────────────────────
load_config() {
  if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
  fi
}

# ── Save config ──────────────────────────────────────────
save_config() {
  cat > "$ENV_FILE" <<EOF
NETWATCH_API_KEY="$NETWATCH_API_KEY"
NETWATCH_ENDPOINT="$NETWATCH_ENDPOINT"
NETWATCH_HOSTNAME="$NETWATCH_HOSTNAME"
NETWATCH_OS="$NETWATCH_OS"
NETWATCH_INTERVAL="${NETWATCH_INTERVAL:-15}"
EOF
  echo "Config saved to $ENV_FILE"
}

# ── Detect host info ─────────────────────────────────────
detect_host() {
  if [ -z "$NETWATCH_HOSTNAME" ]; then
    NETWATCH_HOSTNAME="$(hostname)"
  fi
  if [ -z "$NETWATCH_OS" ]; then
    if command -v sw_vers &>/dev/null; then
      NETWATCH_OS="$(sw_vers -productName) $(sw_vers -productVersion) ($(uname -m))"
    elif [ -f /etc/os-release ]; then
      NETWATCH_OS="$(grep PRETTY_NAME /etc/os-release | cut -d'"' -f2) ($(uname -m))"
    else
      NETWATCH_OS="$(uname -s) $(uname -r) ($(uname -m))"
    fi
  fi
}

# ── Build ────────────────────────────────────────────────
build() {
  echo "Building agent image..."
  docker build -f "$SCRIPT_DIR/Dockerfile.agent" -t "$IMAGE_NAME" "$SCRIPT_DIR" -q
  echo "✅ Built"
}

# ── Run container ────────────────────────────────────────
run_container() {
  docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    -e NETWATCH_API_KEY="$NETWATCH_API_KEY" \
    -e NETWATCH_ENDPOINT="$NETWATCH_ENDPOINT" \
    -e NETWATCH_HOSTNAME="$NETWATCH_HOSTNAME" \
    -e NETWATCH_OS="$NETWATCH_OS" \
    -e NETWATCH_INTERVAL="${NETWATCH_INTERVAL:-15}" \
    "$IMAGE_NAME" > /dev/null
}

# ── Commands ─────────────────────────────────────────────
cmd_start() {
  load_config

  if [ -z "$NETWATCH_API_KEY" ]; then
    echo "First-time setup — enter your API key:"
    read -rp "API Key: " NETWATCH_API_KEY
    read -rp "Endpoint [https://netwatch-api-production.up.railway.app/api/v1/ingest]: " NETWATCH_ENDPOINT
    NETWATCH_ENDPOINT="${NETWATCH_ENDPOINT:-https://netwatch-api-production.up.railway.app/api/v1/ingest}"
    detect_host
    save_config
  fi

  if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
    echo "Agent is already running"
    docker logs --tail 3 "$CONTAINER_NAME" 2>&1
    return
  fi

  docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
  build
  detect_host
  run_container
  sleep 3
  echo "✅ Agent started"
  docker logs --tail 5 "$CONTAINER_NAME" 2>&1
}

cmd_stop() {
  docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
  echo "✅ Agent stopped"
}

cmd_update() {
  load_config

  if [ -z "$NETWATCH_API_KEY" ]; then
    echo "Error: No config found. Run './agent.sh start' first."
    exit 1
  fi

  echo "Updating NetWatch Agent..."
  detect_host
  build
  docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
  run_container
  sleep 3
  echo "✅ Agent updated and restarted"
  docker logs --tail 5 "$CONTAINER_NAME" 2>&1
}

cmd_logs() {
  docker logs -f "$CONTAINER_NAME" 2>&1
}

cmd_status() {
  if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
    echo "✅ Agent is running"
    docker logs --tail 5 "$CONTAINER_NAME" 2>&1
  else
    echo "❌ Agent is not running"
  fi
}

cmd_config() {
  load_config
  echo "Config: $ENV_FILE"
  echo ""
  echo "  API Key:   ${NETWATCH_API_KEY:-(not set)}"
  echo "  Endpoint:  ${NETWATCH_ENDPOINT:-(not set)}"
  echo "  Hostname:  ${NETWATCH_HOSTNAME:-(auto-detect)}"
  echo "  OS:        ${NETWATCH_OS:-(auto-detect)}"
  echo "  Interval:  ${NETWATCH_INTERVAL:-15}s"
}

# ── Main ─────────────────────────────────────────────────
case "${1:-}" in
  start)   cmd_start ;;
  stop)    cmd_stop ;;
  update)  cmd_update ;;
  logs)    cmd_logs ;;
  status)  cmd_status ;;
  config)  cmd_config ;;
  *)
    echo "NetWatch Agent Manager"
    echo ""
    echo "Usage: ./agent.sh <command>"
    echo ""
    echo "Commands:"
    echo "  start    Start the agent (first run prompts for API key)"
    echo "  stop     Stop the agent"
    echo "  update   Rebuild and restart with latest code"
    echo "  logs     Tail agent logs"
    echo "  status   Check if agent is running"
    echo "  config   Show current configuration"
    ;;
esac
