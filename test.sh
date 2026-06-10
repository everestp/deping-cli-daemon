#!/bin/bash

set -e

BINARY_PATH="./target/release/deping"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found: $BINARY_PATH"
    echo "Run: cargo build --release"
    exit 1
fi

echo "📦 Installing deping..."

chmod +x "$BINARY_PATH"

sudo cp "$BINARY_PATH" /usr/local/bin/deping

echo "✅ Installation complete"

echo ""
echo "Version:"
deping --version

echo ""
echo "Try:"
echo "  deping setup"
echo "  deping start"