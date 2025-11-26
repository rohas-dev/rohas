#!/usr/bin/env bash

set -euo pipefail

DEFAULT_INSTALL_DIR="/usr/local/bin"
PYTHON_MAJOR=3
PYTHON_MINOR=12
PYTHON_VERSION="${PYTHON_MAJOR}.${PYTHON_MINOR}"
RUST_MIN_VERSION="1.70"
INSTALL_BINARY="${ROHAS_INSTALL:-false}"

cleanup() {
    if [[ -n "${TMP_DIR:-}" && -d "${TMP_DIR:-}" ]]; then
        rm -rf "${TMP_DIR}"
    fi
}

trap cleanup EXIT

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

require_cmd() {
    if ! command_exists "$1"; then
        echo "Error: required command '$1' not found in PATH." >&2
        exit 1
    fi
}

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64) TARGET="aarch64-apple-darwin";;
                x86_64) TARGET="x86_64-apple-darwin";;
                *) echo "Unsupported macOS architecture: $arch" >&2; exit 1;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64) TARGET="x86_64-unknown-linux-gnu";;
                aarch64|arm64) TARGET="aarch64-unknown-linux-gnu";;
                *) echo "Unsupported Linux architecture: $arch" >&2; exit 1;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            TARGET="x86_64-pc-windows-msvc"
            ;;
        *)
            echo "Unsupported operating system: $os" >&2
            echo "You may still be able to build manually. Check the Rust target for your platform." >&2
            exit 1
            ;;
    esac
}

check_rust() {
    if ! command_exists rustc; then
        echo "Rust is not installed. Installing Rust..."
        if [[ "$(uname -s)" == "Darwin" ]]; then
            if command_exists brew; then
                brew install rust
            else
                echo "Please install Rust manually from https://rustup.rs/" >&2
                exit 1
            fi
        else
            echo "Installing Rust via rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env" || true
        fi
    fi

    if ! command_exists cargo; then
        echo "Cargo is not installed. Please install Rust from https://rustup.rs/" >&2
        exit 1
    fi

    # Check Rust version
    local rust_version
    rust_version=$(rustc --version | cut -d' ' -f2 | cut -d'.' -f1,2)
    local rust_major rust_minor
    rust_major=$(echo "$rust_version" | cut -d'.' -f1)
    rust_minor=$(echo "$rust_version" | cut -d'.' -f2)
    local min_major min_minor
    min_major=$(echo "$RUST_MIN_VERSION" | cut -d'.' -f1)
    min_minor=$(echo "$RUST_MIN_VERSION" | cut -d'.' -f2)

    if [[ "$rust_major" -lt "$min_major" ]] || \
       [[ "$rust_major" -eq "$min_major" && "$rust_minor" -lt "$min_minor" ]]; then
        echo "Rust version $rust_version is too old. Required: $RUST_MIN_VERSION or higher" >&2
        echo "Please update Rust: rustup update stable" >&2
        exit 1
    fi

    echo "Found Rust $(rustc --version)"
}

ensure_python() {
    if command_exists python3; then
        if python3 -c "import sys; sys.exit(0 if sys.version_info >= (${PYTHON_MAJOR}, ${PYTHON_MINOR}) else 1)" 2>/dev/null; then
            echo "Found Python $(python3 --version)"
            return
        fi
    fi

    echo "Python ${PYTHON_VERSION} not found. Attempting to install..."
    if [[ "$(uname -s)" == "Darwin" ]]; then
        if command_exists brew; then
            brew install "python@${PYTHON_MAJOR}.${PYTHON_MINOR}"
        else
            echo "Homebrew is required to install Python ${PYTHON_VERSION}. Install Homebrew and rerun this script." >&2
            exit 1
        fi
    else
        if command_exists apt-get; then
            sudo apt-get update
            sudo apt-get install -y "python${PYTHON_MAJOR}.${PYTHON_MINOR}" "python${PYTHON_MAJOR}.${PYTHON_MINOR}-dev"
        elif command_exists dnf; then
            sudo dnf install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel"
        elif command_exists yum; then
            sudo yum install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel"
        else
            echo "Automatic Python installation is not supported for your distro. Please install Python ${PYTHON_VERSION} manually." >&2
            exit 1
        fi
    fi
}

install_build_dependencies() {
    local os
    os="$(uname -s)"

    case "$os" in
        Linux)
            if command_exists apt-get; then
                echo "Installing build dependencies..."
                sudo apt-get update
                sudo apt-get install -y build-essential pkg-config cmake make perl
            elif command_exists dnf; then
                echo "Installing build dependencies..."
                sudo dnf install -y gcc gcc-c++ pkg-config cmake make perl
            elif command_exists yum; then
                echo "Installing build dependencies..."
                sudo yum install -y gcc gcc-c++ pkg-config cmake make perl
            fi
            ;;
        Darwin)
            if command_exists brew; then
                echo "Installing build dependencies..."
                brew install pkg-config cmake || true
            fi
            ;;
    esac
}

clone_repo() {
    # Check if we're already in a rohas repository
    if [[ -f "Cargo.toml" ]] && grep -q "rohas" "Cargo.toml" 2>/dev/null; then
        echo "Already in rohas repository. Building from current directory."
        return
    fi

    # Check if rohas directory exists in current location
    if [[ -d "rohas" ]]; then
        echo "Directory 'rohas' already exists. Using existing directory."
        cd rohas
        if [[ -d ".git" ]]; then
            echo "Updating repository..."
            git pull || true
        fi
    else
        echo "Cloning rohas repository..."
        git clone https://github.com/rohas-dev/rohas.git
        cd rohas
    fi
}

build_project() {
    echo "Building rohas for target: $TARGET"
    echo "This may take several minutes..."

    # Add target if needed
    if ! rustup target list --installed | grep -q "^${TARGET}$"; then
        echo "Adding Rust target: $TARGET"
        rustup target add "$TARGET"
    fi

    # Set environment variables for vendored OpenSSL
    export OPENSSL_STATIC=1
    export OPENSSL_VENDORED=1

    # Build the project
    cargo build --release --target "$TARGET"

    echo ""
    echo "Build completed successfully!"
    echo "Binary location: target/${TARGET}/release/rohas"
    
    if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
        echo "Binary location: target/${TARGET}/release/rohas.exe"
    fi
}

install_binary() {
    if [[ "$INSTALL_BINARY" != "true" ]]; then
        return
    fi

    local install_dir binary_path
    install_dir="${ROHAS_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
    mkdir -p "$install_dir"

    if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
        binary_path="target/${TARGET}/release/rohas.exe"
    else
        binary_path="target/${TARGET}/release/rohas"
    fi

    if [[ ! -f "$binary_path" ]]; then
        echo "Error: Binary not found at $binary_path" >&2
        exit 1
    fi

    echo "Installing binary to ${install_dir}..."
    if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
        cp "$binary_path" "${install_dir}/rohas.exe"
        echo "Rohas installed to ${install_dir}/rohas.exe"
    else
        chmod +x "$binary_path"
        sudo install -m 0755 "$binary_path" "${install_dir}/rohas"
        echo "Rohas installed to ${install_dir}/rohas"
    fi

    echo ""
    echo "Done! Add ${install_dir} to your PATH if it's not already there."
    echo "You can now run 'rohas --help'."
}

main() {
    echo "Rohas Build Script"
    echo "=================="
    echo ""

    require_cmd git
    detect_platform
    check_rust
    ensure_python
    install_build_dependencies

    # Check if we're already in the rohas repo
    local in_repo=false
    if [[ -f "Cargo.toml" ]] && grep -q "rohas" "Cargo.toml" 2>/dev/null; then
        in_repo=true
        echo "Detected rohas repository in current directory."
    else
        TMP_DIR="$(mktemp -d)"
        cd "$TMP_DIR"
    fi

    clone_repo
    build_project

    if [[ "$INSTALL_BINARY" == "true" ]]; then
        install_binary
    else
        echo ""
        if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
            echo "To install the binary, copy it to a directory in your PATH:"
            echo "  cp target/${TARGET}/release/rohas.exe <path-in-PATH>/rohas.exe"
        else
            echo "To install the binary, run:"
            echo "  sudo install -m 0755 target/${TARGET}/release/rohas ${DEFAULT_INSTALL_DIR}/rohas"
        fi
        echo ""
        echo "Or set ROHAS_INSTALL=true and rerun this script."
    fi
}

main "$@"

