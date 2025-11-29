#!/bin/bash
# create_demo.sh - Create demo GIFs for eunice README
#
# Requirements:
#   - asciinema (apt install asciinema)
#   - agg (cargo install agg) - asciinema GIF generator
#   - eunice installed
#
# Usage: ./create_demo.sh

set -e

# Script is in assets/, project root is parent
ASSETS_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$ASSETS_DIR")"

echo "=== Eunice Demo GIF Generator ==="
echo ""

# Check dependencies
for cmd in asciinema agg eunice; do
    if ! command -v $cmd &> /dev/null; then
        echo "ERROR: $cmd is required but not installed."
        case $cmd in
            asciinema) echo "Install with: apt install asciinema" ;;
            agg) echo "Install with: cargo install agg" ;;
            eunice) echo "Install with: cargo install --path ." ;;
        esac
        exit 1
    fi
done

# Assets directory already exists (we're in it)

echo "Creating demo GIFs in $ASSETS_DIR..."
echo ""

# =============================================================================
# Helper function to create a demo script and record it
# =============================================================================
record_demo() {
    local name=$1
    local cols=$2
    local rows=$3
    local font_size=$4
    local script_file=$5

    local cast_file="/tmp/${name}.cast"
    local gif_file="$ASSETS_DIR/${name}.gif"

    echo "📹 Recording: $name..."

    # Record with asciinema using the script
    # Note: asciinema 2.x doesn't support --cols/--rows, terminal size comes from current term
    COLUMNS=$cols LINES=$rows asciinema rec --overwrite -c "bash $script_file" "$cast_file" 2>/dev/null || \
        asciinema rec --overwrite -c "bash $script_file" "$cast_file"

    # Convert to GIF with specified dimensions
    agg --cols "$cols" --rows "$rows" --font-size "$font_size" --idle-time-limit 3 "$cast_file" "$gif_file"

    # Cleanup
    rm -f "$cast_file" "$script_file"

    echo "✅ Saved to $gif_file"
    echo ""
}

# =============================================================================
# Demo 1: Basic Usage
# =============================================================================
BASIC_SCRIPT="/tmp/demo_basic.sh"
cat > "$BASIC_SCRIPT" << 'SCRIPT'
#!/bin/bash

show() {
    echo -e "\033[32m$\033[0m $1"
    sleep 0.5
}

run() {
    echo -e "\033[32m$\033[0m $1"
    sleep 0.3
    eval "$1"
    sleep 0.8
}

echo ""
echo -e "\033[1;36m# Basic eunice usage\033[0m"
sleep 0.5

run "eunice --version"

echo ""
echo -e "\033[1;36m# Ask a simple question\033[0m"
sleep 0.5

show 'eunice "What is 2 + 2?"'
sleep 0.3
eunice "What is 2 + 2?" 2>&1
sleep 1.5

echo ""
echo -e "\033[1;36m# List available models\033[0m"
sleep 0.5

run "eunice --list-models | head -12"

echo ""
sleep 0.5
SCRIPT
chmod +x "$BASIC_SCRIPT"

record_demo "demo_basic" 90 22 18 "$BASIC_SCRIPT"

# =============================================================================
# Demo 2: DMN Mode
# =============================================================================
# Find a good example directory
if [ -d "$PROJECT_DIR/examples/just_shell_inspection" ]; then
    DMN_EXAMPLE="$PROJECT_DIR/examples/just_shell_inspection"
elif [ -d "$PROJECT_DIR/examples/codebase_archaeologist" ]; then
    DMN_EXAMPLE="$PROJECT_DIR/examples/codebase_archaeologist"
else
    DMN_EXAMPLE="$PROJECT_DIR"
fi

DMN_SCRIPT="/tmp/demo_dmn.sh"
cat > "$DMN_SCRIPT" << SCRIPT
#!/bin/bash

show() {
    echo -e "\033[32m\$\033[0m \$1"
    sleep 0.5
}

run() {
    echo -e "\033[32m\$\033[0m \$1"
    sleep 0.3
    eval "\$1"
    sleep 0.8
}

echo ""
echo -e "\033[1;36m# DMN Mode - Autonomous Batch Execution\033[0m"
sleep 0.5

run "cd $DMN_EXAMPLE && pwd"

run "ls *.md *.toml 2>/dev/null || ls"
sleep 0.5

echo ""
echo -e "\033[1;36m# Run eunice with --dmn flag\033[0m"
sleep 0.5

show 'eunice --dmn "What files are here? Give a 1-line summary"'
sleep 0.3
cd "$DMN_EXAMPLE"
timeout 45 eunice --dmn "What files are here? Give a 1-line summary" 2>&1 | head -40 || true
sleep 1

echo ""
SCRIPT
chmod +x "$DMN_SCRIPT"

record_demo "demo_dmn" 100 28 16 "$DMN_SCRIPT"

# =============================================================================
# Demo 3: Multi-Agent Mode
# =============================================================================
# Clean up any previous artifacts
rm -f "$PROJECT_DIR/examples/real_multi_agent/orders.txt" \
      "$PROJECT_DIR/examples/real_multi_agent/kitchen_log.txt" \
      "$PROJECT_DIR/examples/real_multi_agent/pantry.txt" 2>/dev/null || true

MULTI_SCRIPT="/tmp/demo_multiagent.sh"
cat > "$MULTI_SCRIPT" << SCRIPT
#!/bin/bash

show() {
    echo -e "\033[32m\$\033[0m \$1"
    sleep 0.5
}

run() {
    echo -e "\033[32m\$\033[0m \$1"
    sleep 0.3
    eval "\$1"
    sleep 0.8
}

echo ""
echo -e "\033[1;36m# Multi-Agent Mode - Restaurant Simulation\033[0m"
sleep 0.5

run "cd $PROJECT_DIR/examples/real_multi_agent && pwd"

echo ""
echo -e "\033[1;36m# List configured agents\033[0m"
sleep 0.3

run "eunice --list-agents"

echo ""
echo -e "\033[1;36m# Order food through the multi-agent system\033[0m"
sleep 0.5

show 'eunice "I would like a cheeseburger please"'
sleep 0.3
cd "$PROJECT_DIR/examples/real_multi_agent"
timeout 60 eunice "I would like a cheeseburger please" 2>&1 | head -50 || true
sleep 1

echo ""
echo -e "\033[1;36m# Check generated order file\033[0m"
sleep 0.3

run "cat orders.txt 2>/dev/null | head -5 || echo 'No orders yet'"

echo ""
SCRIPT
chmod +x "$MULTI_SCRIPT"

record_demo "demo_multiagent" 105 32 14 "$MULTI_SCRIPT"

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "=== All demos created! ==="
echo ""
ls -lh "$ASSETS_DIR"/*.gif 2>/dev/null || echo "No GIFs found"
echo ""
echo "Add to README.md:"
echo '  ![Basic Usage](assets/demo_basic.gif)'
echo '  ![DMN Mode](assets/demo_dmn.gif)'
echo '  ![Multi-Agent Mode](assets/demo_multiagent.gif)'
