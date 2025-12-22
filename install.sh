#!/bin/bash
# Install eunice and mcpz from GitHub
# Usage: curl -sSf https://longrunningagents.com/install.sh | bash
#
# This script installs both binaries to ~/.cargo/bin/

set -e

echo "Installing eunice + mcpz from GitHub..."
echo ""

# Check for cargo
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed."
    echo "Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Use system git for SSH key support
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git ssh://git@github.com/xeb/eunice.git --force

echo ""
echo "Installation complete!"
echo "Both 'eunice' and 'mcpz' are now available in ~/.cargo/bin/"
echo ""
echo "Verify with:"
echo "  eunice --version"
echo "  mcpz --version"
