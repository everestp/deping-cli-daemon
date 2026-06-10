#!/bin/bash
set -e

REPO="everestp/deping-cli-daemon"
BINARY="deping"

# OS detection
OS=$(uname -s)
if [[ "$OS" == "Linux" ]]; then
  FILE="$BINARY-linux-amd64"
elif [[ "$OS" == "Darwin" ]]; then
  FILE="$BINARY-macos-amd64"
else
  echo "❌ Unsupported OS: $OS"
  exit 1
fi

URL="https://github.com/$REPO/releases/latest/download/$FILE"

echo "⬇️ Downloading latest $BINARY from GitHub..."

# Use curl to download the file
if ! curl -L "$URL" -o "/tmp/$BINARY"; then
  echo "❌ Failed to download. Ensure your internet is connected."
  exit 1
fi

chmod +x "/tmp/$BINARY"

echo "🚀 Installing $BINARY to /usr/local/bin/..."
sudo mv "/tmp/$BINARY" "/usr/local/bin/$BINARY"

echo "✅ Installed successfully!"
echo "👉 Run: deping setup"