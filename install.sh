#!/bin/bash
# Install eunice and mcpz from GitHub
# Usage: curl -sSf https://longrunningagents.com/install.sh | bash
#
# This script installs both binaries to ~/.cargo/bin/

set -e

VERSION_URL="https://longrunningagents.com/version.txt"

echo "Checking latest version..."

# Get remote version
REMOTE_VERSION=$(curl -sSf "$VERSION_URL" 2>/dev/null | tr -d '[:space:]') || REMOTE_VERSION=""

# Get current version if installed
if command -v eunice &> /dev/null; then
    CURRENT_VERSION=$(eunice --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1) || CURRENT_VERSION=""
else
    CURRENT_VERSION=""
fi

echo "Current version: ${CURRENT_VERSION:-not installed}"
echo "Remote version:  ${REMOTE_VERSION:-unknown}"
echo ""

# Check if update needed
if [ -n "$REMOTE_VERSION" ] && [ "$CURRENT_VERSION" = "$REMOTE_VERSION" ]; then
    echo "Already up to date!"
    exit 0
fi

# Check for cargo
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed."
    echo "Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "Installing eunice + mcpz from GitHub..."
echo ""

# Use system git for SSH key support
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git ssh://git@github.com/xeb/eunice.git --force

echo ""
echo "Installation complete!"
echo "Both 'eunice' and 'mcpz' are now available in ~/.cargo/bin/"
echo ""
echo "Verify with:"
echo "  eunice --version"
echo "  mcpz --version"
