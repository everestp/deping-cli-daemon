#!/usr/bin/env bash

set -Eeuo pipefail

REPO="everestp/deping-cli-daemon"
BINARY="deping"
INSTALL_DIR="/usr/local/bin"
INSTALLER_VERSION="1.0.3"

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
C_MAGENTA="\033[35m"
C_WHITE="\033[97m"

BOX_TL="╭"; BOX_TR="╮"; BOX_BL="╰"; BOX_BR="╯"
BOX_H="─"; BOX_V="│"

# --------------------------------------------------
# UI helpers
# --------------------------------------------------

hr() {
    printf "${C_BLUE}"
    printf '%.0s─' {1..54}
    printf "${C_RESET}\n"
}

print_banner() {
    clear 2>/dev/null || true

    printf "${C_CYAN}${C_BOLD}"
    printf "  ____  _____ ____ ___ _   _  ____ \n"
    printf " |  _ \\| ____|  _ \\_ _| \\ | |/ ___|\n"
    printf " | | | |  _| | |_) | ||  \\| | |  _ \n"
    printf " | |_| | |___|  __/| || |\\  | |_| |\n"
    printf " |____/|_____|_|  |___|_| \\_|\\____|\n"
    printf "${C_RESET}"

    printf "${C_MAGENTA}${C_BOLD}   ⚡ Decentralized Uptime & Edge Node Daemon ⚡${C_RESET}\n"
    printf "  ${C_DIM}${C_WHITE}v${INSTALLER_VERSION}${C_RESET}\n"
    hr
    printf "\n"
}

step_header() {
    local num="$1"
    local total="$2"
    local label="$3"
    printf "  ${C_DIM}┌─[${C_RESET}${C_BOLD}${C_CYAN}%s/%s${C_RESET}${C_DIM}]${C_RESET} ${C_WHITE}${C_BOLD}%s${C_RESET}\n" "$num" "$total" "$label"
}

step_ok() {
    printf "  ${C_DIM}└─${C_RESET} ${C_GREEN}✔${C_RESET} %s\n\n" "$1"
}

step_fail() {
    printf "  ${C_DIM}└─${C_RESET} ${C_RED}✖ %s${C_RESET}\n" "$1"
}

spinner() {
    local pid="$1"
    local msg="$2"
    local spinchars='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    local i=0

    tput civis 2>/dev/null || true

    while kill -0 "$pid" 2>/dev/null; do
        local char="${spinchars:i:1}"
        printf "\r  ${C_DIM}│${C_RESET}  ${C_CYAN}%s${C_RESET}  %s" "$char" "$msg"

        i=$(( (i + 1) % ${#spinchars} ))
        sleep 0.08
    done

    tput cnorm 2>/dev/null || true
    printf "\r\033[K"
}

cleanup() {
    tput cnorm 2>/dev/null || true

    if [[ -n "${TMP_FILE:-}" && -f "$TMP_FILE" ]]; then
        rm -f "$TMP_FILE"
    fi

    if [[ -n "${VERSION_FILE:-}" && -f "$VERSION_FILE" ]]; then
        rm -f "$VERSION_FILE"
    fi
}

trap cleanup EXIT

# --------------------------------------------------
# Banner
# --------------------------------------------------

print_banner

# --------------------------------------------------
# Step 1: System Detection
# --------------------------------------------------

step_header 1 5 "Analyzing system hardware & OS"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64|amd64)
                FILE="${BINARY}-linux-amd64"
                ;;
            aarch64|arm64)
                FILE="${BINARY}-linux-arm64"
                ;;
            *)
                step_fail "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;

    Darwin)
        case "$ARCH" in
            x86_64|amd64)
                FILE="${BINARY}-macos-amd64"
                ;;
            arm64|aarch64)
                FILE="${BINARY}-macos-arm64"
                ;;
            *)
                step_fail "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;

    *)
        step_fail "Unsupported operating system: $OS"
        exit 1
        ;;
esac

sleep 0.3

step_ok "Target identified: ${C_BOLD}${FILE}${C_RESET}${C_GREEN} on ${C_BOLD}${OS}/${ARCH}${C_RESET}"

# --------------------------------------------------
# Step 2: Fetch Latest Release
# --------------------------------------------------

step_header 2 5 "Querying GitHub release metadata"

VERSION_FILE="$(mktemp)"

(
    API_URL="https://api.github.com/repos/${REPO}/releases/latest"

    RELEASE_JSON="$(
        curl \
            --fail \
            --silent \
            --show-error \
            --location \
            --retry 3 \
            --connect-timeout 10 \
            "$API_URL"
    )"

    VERSION_TAG="$(
        printf '%s' "$RELEASE_JSON" |
        grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' |
        head -n 1 |
        sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/'
    )"

    [[ -n "$VERSION_TAG" ]] || exit 1

    printf '%s\n' "$VERSION_TAG" > "$VERSION_FILE"
) &

PID=$!

spinner "$PID" "Fetching latest version tag..."

if ! wait "$PID"; then
    step_fail "Failed to fetch release information."
    exit 1
fi

VERSION_TAG="$(cat "$VERSION_FILE")"

step_ok "Latest version resolved: ${C_BOLD}${C_CYAN}${VERSION_TAG}${C_RESET}"

# --------------------------------------------------
# Step 3: Download
# --------------------------------------------------

URL="https://github.com/${REPO}/releases/download/${VERSION_TAG}/${FILE}"

TMP_FILE="$(mktemp)"

step_header 3 5 "Downloading release binary"
printf "  ${C_DIM}│${C_RESET}\n"

if ! curl \
    --fail \
    --location \
    --retry 3 \
    --connect-timeout 10 \
    --progress-bar \
    "$URL" \
    -o "$TMP_FILE"; then

    printf "\n"
    step_fail "Download failed."
    printf "  ${C_YELLOW}Asset may not exist for ${VERSION_TAG}:${C_RESET}\n"
    printf "  ${C_DIM}%s${C_RESET}\n\n" "$FILE"
    exit 1
fi

if [[ ! -s "$TMP_FILE" ]]; then
    step_fail "Downloaded file is empty."
    exit 1
fi

chmod 755 "$TMP_FILE"

DL_SIZE="$(du -h "$TMP_FILE" 2>/dev/null | cut -f1 || echo "?")"

step_ok "Download completed successfully. (${C_BOLD}${DL_SIZE}${C_RESET}${C_GREEN})"

# --------------------------------------------------
# Step 4: Binary Verification
# --------------------------------------------------

step_header 4 5 "Verifying binary"

if ! "$TMP_FILE" --version >/dev/null 2>&1; then
    step_fail "Binary verification failed."
    printf "  ${C_YELLOW}The binary may be incompatible with this system.${C_RESET}\n"
    exit 1
fi

BINARY_VERSION_TEXT="$(
    "$TMP_FILE" --version 2>/dev/null || echo "$VERSION_TAG"
)"

step_ok "Binary verified: ${C_BOLD}${C_CYAN}${BINARY_VERSION_TEXT}${C_RESET}"

# --------------------------------------------------
# Step 5: Installation
# --------------------------------------------------

step_header 5 5 "Installing into ${INSTALL_DIR}"

if [[ ! -d "$INSTALL_DIR" ]]; then
    sudo mkdir -p "$INSTALL_DIR"
fi

if ! sudo install -m 755 "$TMP_FILE" "${INSTALL_DIR}/${BINARY}"; then
    step_fail "Installation failed."
    printf "  ${C_YELLOW}Administrative permission may be required.${C_RESET}\n"
    exit 1
fi

step_ok "Global command installed successfully!"

# --------------------------------------------------
# Success Dashboard
# --------------------------------------------------

BOX_WIDTH=52

printf "${C_GREEN}${BOX_TL}"
printf '%.0s─' $(seq 1 $BOX_WIDTH)
printf "${BOX_TR}${C_RESET}\n"

printf "${C_GREEN}${BOX_V}${C_RESET}  ${C_BOLD}${C_WHITE}🚀 DePing Edge Daemon is Ready!${C_RESET}%*s${C_GREEN}${BOX_V}${C_RESET}\n" 18 ""

printf "${C_GREEN}${BOX_V}${C_RESET}%*s${C_GREEN}${BOX_V}${C_RESET}\n" $((BOX_WIDTH+1)) ""

printf "${C_GREEN}${BOX_V}${C_RESET}  ${C_DIM}Binary Path${C_RESET}  ${C_CYAN}%-33s${C_RESET}${C_GREEN}${BOX_V}${C_RESET}\n" "${INSTALL_DIR}/${BINARY}"
printf "${C_GREEN}${BOX_V}${C_RESET}  ${C_DIM}Version${C_RESET}      ${C_CYAN}%-33s${C_RESET}${C_GREEN}${BOX_V}${C_RESET}\n" "${BINARY_VERSION_TEXT}"
printf "${C_GREEN}${BOX_V}${C_RESET}  ${C_DIM}Platform${C_RESET}     ${C_CYAN}%-33s${C_RESET}${C_GREEN}${BOX_V}${C_RESET}\n" "${OS}/${ARCH}"

printf "${C_GREEN}${BOX_BL}"
printf '%.0s─' $(seq 1 $BOX_WIDTH)
printf "${BOX_BR}${C_RESET}\n\n"

# --------------------------------------------------
# Setup Prompt
# --------------------------------------------------

read -r -p "  ⚡ Would you like to run 'deping setup' now? [Y/n] " response

response="${response:-Y}"

if [[ "$response" =~ ^[Yy]$ ]]; then
    printf "\n"
    exec "${INSTALL_DIR}/${BINARY}" setup
else
    printf "\n"
    printf "  ${C_CYAN}To get started later:${C_RESET}\n\n"
    printf "    ${C_BOLD}deping setup${C_RESET}\n"
    printf "    ${C_BOLD}deping start${C_RESET}\n\n"
fi

hr
printf "\n"
