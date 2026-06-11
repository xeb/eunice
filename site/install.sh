#!/usr/bin/env bash
# eunice installer — https://longrunningagents.com
# Usage: curl -sSf https://longrunningagents.com/install.sh | bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found — installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi

echo "Installing eunice from https://github.com/xeb/eunice ..."
cargo install --git https://github.com/xeb/eunice.git --force

echo
echo "Done. Run 'eunice --version' to verify."
