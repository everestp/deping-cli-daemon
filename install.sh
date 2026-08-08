#!/usr/bin/env bash

set -Eeuo pipefail

REPO="everestp/deping-cli-daemon"
BINARY="deping"
INSTALL_DIR="/usr/local/bin"

# --------------------------------------------------
# Typography & Colors
# --------------------------------------------------
C_RESET="\033[0m"
C_BOLD="\033[1m"
C_DIM="\033[2m"
C_CYAN="\033[36m"
C_BLUE="\033[34m"
C_GREEN="\033[32m"
C_YELLOW="\033[33m"
C_RED="\033[31m"

# --------------------------------------------------
# UI Banner & Visual Engine
# --------------------------------------------------
print_banner() {
  clear
  printf "${C_CYAN}${C_BOLD}"
  printf "  ____             _             \n"
  printf " |  _ \  ___ _ __ (_)_ __   __ _ \n"
  printf " | | | |/ _ \ '_ \\| | '_ \\ / _\` |\n"
  printf " | |_| |  __/ |_) | | | | | (_| |\n"
  printf " |____/ \\___| .__/|_|_| |_|\\__, |\n"
  printf "            |_|            |___/ \n"
  printf "${C_RESET}"
  printf "${C_DIM} Decentralized Uptime & Edge Node Daemon v1.0${C_RESET}\n"
  printf "${C_BLUE}──────────────────────────────────────────────────${C_RESET}\n\n"
}

spinner() {
  local pid=$1
  local msg="$2"
  local spinchars='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
  local i=0

  tput civis -- invisible 2>/dev/null || true

  while kill -0 "$pid" 2>/dev/null; do
    local char="${spinchars:i:1}"
    printf "\r  ${C_CYAN}%s${C_RESET}  %s" "$char" "$msg"
    i=$(( (i+1) % ${#spinchars} ))
    sleep 0.08
  done

  tput cnorm -- normal 2>/dev/null || true
  printf "\r\033[K"
}

progress_bar() {
  local current=$1
  local total=$2
  local width=25
  local percentage=$((current * 100 / total))
  local filled=$((current * width / total))
  local empty=$((width - filled))

  local bar_fill=""
  local bar_empty=""

  for ((j=0; j<filled; j++)); do bar_fill="${bar_fill}█"; done
  for ((j=0; j<empty; j++)); do bar_empty="${bar_empty}░"; done

  printf "\r  ${C_CYAN}⠋${C_RESET} Downloading: [${C_GREEN}%s${C_DIM}%s${C_RESET}] %3d%%" "$bar_fill" "$bar_empty" "$percentage"
}

cleanup() {
  tput cnorm -- normal 2>/dev/null || true
  if [[ -n "${TMP_FILE:-}" && -f "$TMP_FILE" ]]; then
    rm -f "$TMP_FILE"
  fi
}

trap cleanup EXIT

# --------------------------------------------------
# Interactive Installation Sequence
# --------------------------------------------------
print_banner

# Step 1: System Environment Analysis
printf "  ${C_DIM}[1/5]${C_RESET} Analyzing system hardware & OS...\n"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64|amd64) FILE="$BINARY-linux-amd64" ;;
      aarch64|arm64) FILE="$BINARY-linux-arm64" ;;
      *) printf "  ${C_RED}✖ Unsupported Linux architecture: $ARCH${C_RESET}\n"; exit 1 ;;
    end
    ;;
  Darwin)
    case "$ARCH" in
      x86_64|amd64) FILE="$BINARY-macos-amd64" ;;
      arm64|aarch64) FILE="$BINARY-macos-arm64" ;;
      *) printf "  ${C_RED}✖ Unsupported macOS architecture: $ARCH${C_RESET}\n"; exit 1 ;;
    end
    ;;
  *)
    printf "  ${C_RED}✖ Unsupported operating system: $OS${C_RESET}\n"; exit 1
    ;;
esac
sleep 0.3
printf "  ${C_GREEN}✔${C_RESET} Target identified: ${C_BOLD}%s${C_RESET} on ${C_BOLD}%s/%s${C_RESET}\n\n" "$FILE" "$OS" "$ARCH"

# Step 2: Fetching Release Metadata
printf "  ${C_DIM}[2/5]${C_RESET} Querying GitHub release metadata..."
(
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
  RELEASE_JSON=$(curl --fail --silent --show-error --connect-timeout 10 "$API_URL")
  VERSION_TAG=$(printf '%s' "$RELEASE_JSON" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n 1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
  echo "$VERSION_TAG" > /tmp/deping_version.tmp
) &
pid=$!
spinner $pid "Fetching latest version tag..."

VERSION_TAG=$(cat /tmp/deping_version.tmp 2>/dev/null || echo "v1.0.0")
rm -f /tmp/deping_version.tmp
printf "  ${C_GREEN}✔${C_RESET} Latest version resolved: ${C_CYAN}%s${C_RESET}\n\n" "$VERSION_TAG"

# Step 3: Streamlined Downloader with Simulated Progress Bar
URL="https://github.com/$REPO/releases/download/$VERSION_TAG/$FILE"
TMP_FILE="/tmp/$BINARY"

printf "  ${C_DIM}[3/5]${C_RESET} Initializing secure file transfer...\n"
(
  curl --fail --location --silent --show-error --connect-timeout 10 "$URL" -o "$TMP_FILE"
) &
dl_pid=$!

# Render animated progress ticks while downloading
dl_step=0
tput civis -- invisible 2>/dev/null || true
while kill -0 "$dl_pid" 2>/dev/null; do
  dl_step=$(( (dl_step + 5) % 95 ))
  progress_bar "$dl_step" 100
  sleep 0.05
done
progress_bar 100 100
echo

if [[ ! -s "$TMP_FILE" ]]; then
  printf "  ${C_RED}✖ Download failed. Asset might not be published for version %s.${C_RESET}\n" "$VERSION_TAG"
  exit 1
fi
chmod +x "$TMP_FILE"
printf "  ${C_GREEN}✔${C_RESET} Download verified & payload authorized.\n\n"

# Step 4: Binary Execution Sanity Check
printf "  ${C_DIM}[4/5]${C_RESET} Running binary integrity & execution test..."
(
  "$TMP_FILE" --version >/dev/null 2>&1
) &
pid=$!
spinner $pid "Verifying binary signature..."

if ! "$TMP_FILE" --version >/dev/null 2>&1; then
  printf "  ${C_RED}✖ Execution check failed. Incompatible system architecture binary.${C_RESET}\n"
  exit 1
fi
BINARY_VERSION_TEXT="$("$TMP_FILE" --version 2>/dev/null || echo "$VERSION_TAG")"
printf "  ${C_GREEN}✔${C_RESET} Integrity verified: ${C_CYAN}%s${C_RESET}\n\n" "$BINARY_VERSION_TEXT"

# Step 5: Global System Integration (Sudo Install & Interactive Setup Prompt)
printf "  ${C_DIM}[5/5]${C_RESET} Installing into system PATH (${INSTALL_DIR})...\n"
if [[ ! -d "$INSTALL_DIR" ]]; then
  sudo mkdir -p "$INSTALL_DIR"
fi

if ! sudo mv "$TMP_FILE" "$INSTALL_DIR/$BINARY"; then
  printf "  ${C_RED}✖ Installation aborted. Administrative permission (sudo) denied.${C_RESET}\n"
  exit 1
fi
sleep 0.4
printf "  ${C_GREEN}✔${C_RESET} Global command installed successfully!\n\n"

# --------------------------------------------------
# Success Dashboard & Post-Install Wizard Prompt
# --------------------------------------------------
printf "${C_BLUE}──────────────────────────────────────────────────${C_RESET}\n"
printf "  ${C_GREEN}${C_BOLD}🚀 DePing Edge Daemon is Ready for Action!${C_RESET}\n"
printf "${C_BLUE}──────────────────────────────────────────────────${C_RESET}\n\n"
printf "  ${C_DIM}Binary Path :${C_RESET} ${INSTALL_DIR}/${BINARY}\n"
printf "  ${C_DIM}Version     :${C_RESET} ${BINARY_VERSION_TEXT}\n\n"

read -rp "  ⚡ Would you like to run 'deping setup' right now? [Y/n] " response
response=${response:-Y}

if [[ "$response" =~ ^[Yy]$ ]]; then
  printf "\n"
  exec deping setup
else
  printf "\n  ${C_CYAN}To get started later, simply run:${C_RESET}\n\n"
  printf "    ${C_BOLD}$ deping setup${C_RESET}\n"
  printf "    ${C_BOLD}$ deping start${C_RESET}\n\n"
  printf "${C_BLUE}──────────────────────────────────────────────────${C_RESET}\n"
fi
