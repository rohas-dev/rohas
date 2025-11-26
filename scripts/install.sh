#!/usr/bin/env bash

set -euo pipefail

DEFAULT_INSTALL_DIR="/usr/local/bin"
PYTHON_MAJOR=3
PYTHON_MINOR=12
PYTHON_VERSION="${PYTHON_MAJOR}.${PYTHON_MINOR}"
TMP_DIR=""

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
                arm64) TARGET="rohas-macos-arm64";;
                x86_64) TARGET="rohas-macos-x86_64";;
                *) echo "Unsupported macOS architecture: $arch" >&2; exit 1;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64) TARGET="rohas-linux-x86_64";;
                aarch64|arm64) TARGET="rohas-linux-arm64";;
                *) echo "Unsupported Linux architecture: $arch" >&2; exit 1;;
            esac
            ;;
        *)
            echo "Unsupported operating system: $os" >&2
            exit 1
            ;;
    esac
}

ensure_python() {
    if command_exists python3; then
        if python3 -c "import sys; sys.exit(0 if sys.version_info >= (${PYTHON_MAJOR}, ${PYTHON_MINOR}) else 1)"; then
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

resolve_version() {
    if [[ -n "${ROHAS_VERSION:-}" ]]; then
        RELEASE_TAG="${ROHAS_VERSION}"
        return
    fi

    echo "Fetching latest release information..."
    local api_url="https://api.github.com/repos/rohas-dev/rohas/releases/latest"
    RELEASE_TAG="$(curl -fsSL "$api_url" | grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')"

    if [[ -z "$RELEASE_TAG" ]]; then
        echo "Failed to determine the latest release tag." >&2
        exit 1
    fi
}

download_and_install() {
    local install_dir asset download_url

    install_dir="${ROHAS_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
    mkdir -p "$install_dir"

    asset="${TARGET}.tar.gz"
    case "$TARGET" in
        *windows*)
            asset="${TARGET}.zip"
            ;;
    esac

    download_url="https://github.com/rohas-dev/rohas/releases/download/${RELEASE_TAG}/${asset}"

    echo "Downloading ${asset} (${RELEASE_TAG})..."
    TMP_DIR="$(mktemp -d)"

    curl -fsSL "$download_url" -o "${TMP_DIR}/${asset}"

    echo "Extracting binary..."
    if [[ "$asset" == *.tar.gz ]]; then
        tar -xzf "${TMP_DIR}/${asset}" -C "$TMP_DIR"
    else
        require_cmd unzip
        unzip -q "${TMP_DIR}/${asset}" -d "$TMP_DIR"
    fi

    if [[ ! -f "${TMP_DIR}/rohas" ]]; then
        echo "Failed to locate rohas binary inside the archive." >&2
        exit 1
    fi

    chmod +x "${TMP_DIR}/rohas"
    sudo install -m 0755 "${TMP_DIR}/rohas" "${install_dir}/rohas"

    echo "Rohas installed to ${install_dir}/rohas"
}

main() {
    require_cmd curl
    detect_platform
    ensure_python
    resolve_version
    download_and_install

    echo ""
    echo "Done! Add ${DEFAULT_INSTALL_DIR} to your PATH if it's not already there."
    echo "You can now run 'rohas --help'."
}

main "$@"

