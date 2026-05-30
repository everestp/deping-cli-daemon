#!/bin/bash

set -e

REPO="everestp/deping"
BINARY="deping"

OS=$(uname -s)
ARCH=$(uname -m)

if [[ "$OS" == "Linux" ]]; then
  FILE="$BINARY-linux-amd64"
elif [[ "$OS" == "Darwin" ]]; then
  FILE="$BINARY-macos-amd64"
else
  echo "Unsupported OS"
  exit 1
fi

URL="https://github.com/$REPO/releases/latest/download/$FILE"

echo "Downloading $BINARY..."

curl -L $URL -o /tmp/$BINARY
chmod +x /tmp/$BINARY
sudo mv /tmp/$BINARY /usr/local/bin/$BINARY

echo "Installed successfully!"
echo "Run: deping setup"
