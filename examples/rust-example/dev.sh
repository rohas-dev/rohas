#!/bin/bash
# Development helper script for Rohas developers
# For end users: install rohas CLI and run "rohas dev --workbench" directly

set -e

# Find the workspace root (look for Cargo.toml with [workspace])
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$SCRIPT_DIR"

# Look for workspace root (go up to 10 levels to handle nested examples)
for i in {1..10}; do
    if [ -f "$WORKSPACE_ROOT/Cargo.toml" ]; then
        # Check if it's a workspace (has [workspace] and contains crates/rohas-cli)
        if grep -q "^\[workspace\]" "$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && \
            [ -d "$WORKSPACE_ROOT/crates/rohas-cli" ]; then
            break
        fi
    fi
    WORKSPACE_ROOT="$(dirname "$WORKSPACE_ROOT")"
    # Stop if we've reached the filesystem root
    if [ "$WORKSPACE_ROOT" = "/" ] || [ "$WORKSPACE_ROOT" = "$SCRIPT_DIR" ]; then
        break
    fi
done

if [ -f "$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && \
   [ -d "$WORKSPACE_ROOT/crates/rohas-cli" ]; then
    cd "$WORKSPACE_ROOT"
    REL_SCHEMA_PATH=$(python3 -c "import os; print(os.path.relpath('$SCRIPT_DIR/schema', '$WORKSPACE_ROOT'))" 2>/dev/null || \
                      perl -MFile::Spec -e "print File::Spec->abs2rel('$SCRIPT_DIR/schema', '$WORKSPACE_ROOT')" 2>/dev/null || \
                      echo "schema")
    # Check if --schema argument is already provided
    HAS_SCHEMA_ARG=false
    for arg in "$@"; do
        if [[ "$arg" == "--schema" ]] || [[ "$arg" == "-s" ]]; then
            HAS_SCHEMA_ARG=true
            break
        fi
    done
    # If no schema arg provided, add it
    if [ "$HAS_SCHEMA_ARG" = false ]; then
        exec cargo run -p rohas-cli -- dev --schema "$REL_SCHEMA_PATH" "$@"
    else
        exec cargo run -p rohas-cli -- dev "$@"
    fi
else
    # Not in workspace - try installed binary or show helpful error
    if command -v rohas >/dev/null 2>&1; then
        cd "$SCRIPT_DIR"
        exec rohas dev "$@"
    else
        echo "Error: Could not find Rohas workspace root and rohas CLI is not installed"
        echo ""
        echo "For Rohas developers: Run this script from within the rohas workspace"
        echo "For end users: Install rohas CLI first:"
        echo "  cargo install --path <path-to-rohas>/crates/rohas-cli"
        echo "  Then run: rohas dev --workbench"
        exit 1
    fi
fi
