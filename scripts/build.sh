#!/usr/bin/env bash

set -euo pipefail

DEFAULT_INSTALL_DIR="/usr/local/bin"

DEFAULT_WORKBENCH_DIR="${HOME}/.rohas"
PYTHON_MAJOR=3
PYTHON_MINOR=12
PYTHON_VERSION="${PYTHON_MAJOR}.${PYTHON_MINOR}"
RUST_MIN_VERSION="1.70"
INSTALL_BINARY="${ROHAS_INSTALL:-true}"
INSTALL_WORKBENCH="${ROHAS_INSTALL_WORKBENCH:-true}"
ROHAS_VERSION="${ROHAS_VERSION:-}"

VERBOSE=false
for arg in "$@"; do
    case "$arg" in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --version=*|--tag=*)
            ROHAS_VERSION="${arg#*=}"
            shift
            ;;
        --version|-V|--tag)
            shift
            if [[ $# -gt 0 ]]; then
                ROHAS_VERSION="$1"
                shift
            else
                echo "Error: --version/--tag requires an argument" >&2
                exit 1
            fi
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -v, --verbose            Show detailed logs and progress"
            echo "  --version, --tag <tag>   Build from a specific git tag/branch (or set ROHAS_VERSION)"
            echo "  -h, --help               Show this help message"
            exit 0
            ;;
    esac
done

cleanup() {
    if [[ -n "${TMP_DIR:-}" && -d "${TMP_DIR:-}" ]]; then
        rm -rf "${TMP_DIR}"
    fi
}

trap cleanup EXIT

log_info() {
    echo "$@"
}

log_verbose() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo "$@" >&2
    fi
}

log_error() {
    echo "$@" >&2
}

show_progress() {
    local message="$1"
    local pid="$2"
    
    if [[ "$VERBOSE" == "true" ]]; then
        wait "$pid"
        return $?
    fi
    
    local spin='-\|/'
    local i=0
    while kill -0 "$pid" 2>/dev/null; do
        i=$(( (i+1) %4 ))
        printf "\r${message} ${spin:$i:1}" >&2
        sleep 0.1
    done
    printf "\r${message} ✓\n" >&2
    wait "$pid"
    return $?
}

run_with_output() {
    local message="$1"
    shift
    
    if [[ "$VERBOSE" == "true" ]]; then
        log_info "$message"
        "$@"
    else
        printf "${message}... " >&2
        "$@" >/dev/null 2>&1 &
        local pid=$!
        show_progress "$message" "$pid"
    fi
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

require_cmd() {
    if ! command_exists "$1"; then
        log_error "Error: required command '$1' not found in PATH."
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
                *) log_error "Unsupported macOS architecture: $arch"; exit 1;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64) TARGET="x86_64-unknown-linux-gnu";;
                aarch64|arm64) TARGET="aarch64-unknown-linux-gnu";;
                *) log_error "Unsupported Linux architecture: $arch"; exit 1;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            TARGET="x86_64-pc-windows-msvc"
            ;;
        *)
            log_error "Unsupported operating system: $os"
            log_error "You may still be able to build manually. Check the Rust target for your platform."
            exit 1
            ;;
    esac
}

check_rust() {
    if ! command_exists rustc; then
        log_info "Rust is not installed. Installing Rust..."
        if [[ "$(uname -s)" == "Darwin" ]]; then
            if command_exists brew; then
                brew install rust
            else
                log_error "Please install Rust manually from https://rustup.rs/"
                exit 1
            fi
        else
            log_info "Installing Rust via rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env" || true
        fi
    fi

    if ! command_exists cargo; then
        log_error "Cargo is not installed. Please install Rust from https://rustup.rs/"
        exit 1
    fi

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
        log_error "Rust version $rust_version is too old. Required: $RUST_MIN_VERSION or higher"
        log_error "Please update Rust: rustup update stable"
        exit 1
    fi

    log_info "Found Rust $(rustc --version)"
}

check_node() {
    if ! command_exists node; then
        log_info "Node.js is not installed. Installing Node.js..."
        if [[ "$(uname -s)" == "Darwin" ]]; then
            if command_exists brew; then
                brew install node
            else
                log_error "Please install Node.js manually from https://nodejs.org/"
                exit 1
            fi
        else
            if command_exists apt-get; then
                if [[ "$VERBOSE" == "true" ]]; then
                    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
                    sudo apt-get install -y nodejs
                else
                    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - >/dev/null 2>&1
                    sudo apt-get install -y nodejs >/dev/null 2>&1
                fi
            elif command_exists dnf; then
                if [[ "$VERBOSE" == "true" ]]; then
                    curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
                    sudo dnf install -y nodejs
                else
                    curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash - >/dev/null 2>&1
                    sudo dnf install -y nodejs >/dev/null 2>&1
                fi
            elif command_exists yum; then
                if [[ "$VERBOSE" == "true" ]]; then
                    curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
                    sudo yum install -y nodejs
                else
                    curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash - >/dev/null 2>&1
                    sudo yum install -y nodejs >/dev/null 2>&1
                fi
            else
                log_error "Please install Node.js manually from https://nodejs.org/"
                exit 1
            fi
        fi
    fi

    if ! command_exists pnpm; then
        log_info "pnpm is not installed. Installing pnpm..."
        if command_exists npm; then
            if [[ "$VERBOSE" == "true" ]]; then
                npm install -g pnpm
            else
                npm install -g pnpm >/dev/null 2>&1
            fi
        else
            log_error "npm is required to install pnpm. Please install Node.js first."
            exit 1
        fi
    fi

    log_info "Found Node.js $(node --version) and pnpm $(pnpm --version)"
}

ensure_python() {
    if command_exists python3; then
        if python3 -c "import sys; sys.exit(0 if sys.version_info >= (${PYTHON_MAJOR}, ${PYTHON_MINOR}) else 1)" 2>/dev/null; then
            log_info "Found Python $(python3 --version)"
            return
        fi
    fi

    log_info "Python ${PYTHON_VERSION} not found. Attempting to install..."
    if [[ "$(uname -s)" == "Darwin" ]]; then
        if command_exists brew; then
            if [[ "$VERBOSE" == "true" ]]; then
                brew install "python@${PYTHON_MAJOR}.${PYTHON_MINOR}"
            else
                brew install "python@${PYTHON_MAJOR}.${PYTHON_MINOR}" >/dev/null 2>&1
            fi
        else
            log_error "Homebrew is required to install Python ${PYTHON_VERSION}. Install Homebrew and rerun this script."
            exit 1
        fi
    else
        if command_exists apt-get; then
            if [[ "$VERBOSE" == "true" ]]; then
                sudo apt-get update
                sudo apt-get install -y "python${PYTHON_MAJOR}.${PYTHON_MINOR}" "python${PYTHON_MAJOR}.${PYTHON_MINOR}-dev"
            else
                sudo apt-get update >/dev/null 2>&1
                sudo apt-get install -y "python${PYTHON_MAJOR}.${PYTHON_MINOR}" "python${PYTHON_MAJOR}.${PYTHON_MINOR}-dev" >/dev/null 2>&1
            fi
        elif command_exists dnf; then
            if [[ "$VERBOSE" == "true" ]]; then
                sudo dnf install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel"
            else
                sudo dnf install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel" >/dev/null 2>&1
            fi
        elif command_exists yum; then
            if [[ "$VERBOSE" == "true" ]]; then
                sudo yum install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel"
            else
                sudo yum install -y "python${PYTHON_MAJOR}${PYTHON_MINOR}" "python${PYTHON_MAJOR}${PYTHON_MINOR}-devel" >/dev/null 2>&1
            fi
        else
            log_error "Automatic Python installation is not supported for your distro. Please install Python ${PYTHON_VERSION} manually."
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
                if [[ "$VERBOSE" == "true" ]]; then
                    log_info "Installing build dependencies..."
                    sudo apt-get update
                    sudo apt-get install -y build-essential pkg-config cmake make perl
                else
                    printf "Installing build dependencies... " >&2
                    sudo apt-get update >/dev/null 2>&1
                    sudo apt-get install -y build-essential pkg-config cmake make perl >/dev/null 2>&1 &
                    local deps_pid=$!
                    show_progress "Installing build dependencies" "$deps_pid"
                fi
            elif command_exists dnf; then
                if [[ "$VERBOSE" == "true" ]]; then
                    log_info "Installing build dependencies..."
                    sudo dnf install -y gcc gcc-c++ pkg-config cmake make perl
                else
                    printf "Installing build dependencies... " >&2
                    sudo dnf install -y gcc gcc-c++ pkg-config cmake make perl >/dev/null 2>&1 &
                    local deps_pid=$!
                    show_progress "Installing build dependencies" "$deps_pid"
                fi
            elif command_exists yum; then
                if [[ "$VERBOSE" == "true" ]]; then
                    log_info "Installing build dependencies..."
                    sudo yum install -y gcc gcc-c++ pkg-config cmake make perl
                else
                    printf "Installing build dependencies... " >&2
                    sudo yum install -y gcc gcc-c++ pkg-config cmake make perl >/dev/null 2>&1 &
                    local deps_pid=$!
                    show_progress "Installing build dependencies" "$deps_pid"
                fi
            fi
            ;;
        Darwin)
            if command_exists brew; then
                if [[ "$VERBOSE" == "true" ]]; then
                    log_info "Installing build dependencies..."
                    brew install pkg-config cmake || true
                else
                    printf "Installing build dependencies... " >&2
                    brew install pkg-config cmake >/dev/null 2>&1 || true
                    printf "✓\n" >&2
                fi
            fi
            ;;
    esac
}

clone_repo() {
    # Check if we're already in a rohas repository
    if [[ -f "Cargo.toml" ]] && grep -q "rohas" "Cargo.toml" 2>/dev/null; then
        log_verbose "Already in rohas repository. Building from current directory."
        if [[ -n "${ROHAS_VERSION}" ]]; then
            log_info "Checking out rohas version: ${ROHAS_VERSION}"
            if [[ "$VERBOSE" == "true" ]]; then
                git fetch --tags || true
                git checkout "${ROHAS_VERSION}" || git checkout "tags/${ROHAS_VERSION}" || {
                    log_error "Failed to checkout version ${ROHAS_VERSION}"
                    exit 1
                }
            else
                git fetch --tags >/dev/null 2>&1 || true
                git checkout "${ROHAS_VERSION}" >/dev/null 2>&1 \
                    || git checkout "tags/${ROHAS_VERSION}" >/dev/null 2>&1 \
                    || {
                        log_error "Failed to checkout version ${ROHAS_VERSION}"
                        exit 1
                    }
            fi
        fi
        return
    fi

    # Check if rohas directory exists in current location
    if [[ -d "rohas" ]]; then
        log_verbose "Directory 'rohas' already exists. Using existing directory."
        cd rohas
        if [[ -d ".git" ]]; then
            if [[ "$VERBOSE" == "true" ]]; then
                log_info "Updating repository..."
                git pull || true
            else
                git pull >/dev/null 2>&1 || true
            fi
        fi
    else
        if [[ "$VERBOSE" == "true" ]]; then
            log_info "Cloning rohas repository..."
            git clone https://github.com/rohas-dev/rohas.git
        else
            printf "Cloning repository... " >&2
            git clone https://github.com/rohas-dev/rohas.git >/dev/null 2>&1 &
            local clone_pid=$!
            show_progress "Cloning repository" "$clone_pid"
        fi
        cd rohas

        if [[ -n "${ROHAS_VERSION}" ]]; then
            log_info "Checking out rohas version: ${ROHAS_VERSION}"
            if [[ "$VERBOSE" == "true" ]]; then
                git fetch --tags || true
                git checkout "${ROHAS_VERSION}" || git checkout "tags/${ROHAS_VERSION}" || {
                    log_error "Failed to checkout version ${ROHAS_VERSION}"
                    exit 1
                }
            else
                git fetch --tags >/dev/null 2>&1 || true
                git checkout "${ROHAS_VERSION}" >/dev/null 2>&1 \
                    || git checkout "tags/${ROHAS_VERSION}" >/dev/null 2>&1 \
                    || {
                        log_error "Failed to checkout version ${ROHAS_VERSION}"
                        exit 1
                    }
            fi
        fi
    fi
}

build_project() {
    log_info "Building rohas for target: $TARGET"
    if [[ "$VERBOSE" != "true" ]]; then
        log_info "This may take several minutes..."
    fi

    # Add target if needed
    if ! rustup target list --installed | grep -q "^${TARGET}$"; then
        log_verbose "Adding Rust target: $TARGET"
        if [[ "$VERBOSE" == "true" ]]; then
            rustup target add "$TARGET"
        else
            rustup target add "$TARGET" >/dev/null 2>&1
        fi
    fi

    # Set environment variables for vendored OpenSSL
    export OPENSSL_STATIC=1
    export OPENSSL_VENDORED=1

    # Build the project
    if [[ "$VERBOSE" == "true" ]]; then
        log_info "Running cargo build..."
        cargo build --release --target "$TARGET"
    else
        printf "Building Rust project... " >&2
        cargo build --release --target "$TARGET" >/dev/null 2>&1 &
        local build_pid=$!
        show_progress "Building Rust project" "$build_pid"
    fi

    log_info ""
    log_info "Build completed successfully!"
    log_verbose "Binary location: target/${TARGET}/release/rohas"
    
    if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
        log_verbose "Binary location: target/${TARGET}/release/rohas.exe"
    fi
}

build_workbench() {
    if [[ "$INSTALL_WORKBENCH" != "true" ]]; then
        return
    fi

    local repo_root
    repo_root="$(pwd)"
    
    if [[ ! -d "workbench" ]]; then
        log_error "Warning: workbench directory not found. Skipping workbench build."
        return
    fi

    if [[ ! -f "workbench/package.json" ]]; then
        log_error "Warning: workbench/package.json not found. Skipping workbench build."
        return
    fi

    log_info "Building workbench..."
    
    local workbench_dir="${repo_root}/workbench"
    cd "$workbench_dir" || {
        log_error "Failed to change to workbench directory"
        return
    }

    if [[ "$VERBOSE" == "true" ]]; then
        log_info "Installing workbench dependencies..."
        pnpm install --frozen-lockfile || {
            log_error "Warning: Failed to install workbench dependencies. Continuing..."
            cd "$repo_root" || true
            return
        }
    else
        printf "Installing workbench dependencies... " >&2
        pnpm install --frozen-lockfile >/dev/null 2>&1 &
        local install_pid=$!
        if ! show_progress "Installing workbench dependencies" "$install_pid"; then
            log_error "Warning: Failed to install workbench dependencies. Continuing..."
            cd "$repo_root" || true
            return
        fi
    fi

    if [[ "$VERBOSE" == "true" ]]; then
        log_info "Building workbench (this may take a few minutes)..."
        pnpm build || {
            log_error "Warning: Failed to build workbench. Continuing..."
            cd "$repo_root" || true
            return
        }
    else
        printf "Building workbench... " >&2
        pnpm build >/dev/null 2>&1 &
        local build_pid=$!
        if ! show_progress "Building workbench" "$build_pid"; then
            log_error "Warning: Failed to build workbench. Continuing..."
            cd "$repo_root" || true
            return
        fi
    fi

    if [[ ! -d ".next" ]] && [[ ! -d "out" ]]; then
        log_error "Warning: Workbench build did not produce .next or out directory. Build may have failed."
        cd "$repo_root" || true
        return
    fi

    cd "$repo_root" || true
    log_info "Workbench built successfully!"
}

install_workbench() {
    if [[ "$INSTALL_WORKBENCH" != "true" ]]; then
        return
    fi

    if [[ ! -d "workbench" ]] || ([[ ! -d "workbench/.next" ]] && [[ ! -d "workbench/out" ]]); then
        log_error "Warning: Workbench not built. Skipping workbench installation."
        return
    fi

    local workbench_install_dir
    if [[ "$(uname -s)" == "Darwin" ]] || [[ "$(uname -s)" == "Linux" ]]; then
        workbench_install_dir="${ROHAS_WORKBENCH_DIR:-$DEFAULT_WORKBENCH_DIR}"
    else
        return
    fi

    log_info "Installing workbench to ${workbench_install_dir}/workbench..."
    sudo mkdir -p "${workbench_install_dir}"
    
    if [[ -d "${workbench_install_dir}/workbench" ]]; then
        log_verbose "Removing existing workbench installation..."
        sudo rm -rf "${workbench_install_dir}/workbench"
    fi

    local workbench_source
    workbench_source="$(pwd)/workbench"
    
    if [[ ! -d "$workbench_source" ]]; then
        log_error "Error: Workbench source directory not found at $workbench_source"
        return
    fi

    if [[ "$VERBOSE" == "true" ]]; then
        log_info "Copying workbench files from $workbench_source..."
        sudo cp -R "$workbench_source" "${workbench_install_dir}/"
    else
        printf "Installing workbench files... " >&2
        sudo cp -R "$workbench_source" "${workbench_install_dir}/" >/dev/null 2>&1 &
        local copy_pid=$!
        show_progress "Installing workbench files" "$copy_pid"
    fi

    if [[ -d "${workbench_install_dir}/workbench/node_modules" ]]; then
        log_verbose "Removing copied node_modules in installed workbench..."
        sudo rm -rf "${workbench_install_dir}/workbench/node_modules"
    fi
    if [[ -d "${workbench_install_dir}/workbench/.next" ]]; then
        log_verbose "Removing copied .next in installed workbench..."
        sudo rm -rf "${workbench_install_dir}/workbench/.next"
    fi

    local current_user="${USER:-$(whoami)}"
    local current_group
    current_group="$(id -gn "$current_user" 2>/dev/null || echo "$current_user")"
    
    log_verbose "Setting permissions for workbench installation..."
    
    sudo chown -R "$current_user:$current_group" "${workbench_install_dir}/workbench" 2>/dev/null || true
    
    sudo chmod -R u+w "${workbench_install_dir}/workbench" 2>/dev/null || true
 
    if [[ -d "${workbench_install_dir}/workbench/node_modules" ]]; then
        sudo chmod -R u+rX "${workbench_install_dir}/workbench/node_modules" 2>/dev/null || true
    fi
 
    if [[ -d "${workbench_install_dir}/workbench/.next" ]]; then
        sudo chown -R "$current_user:$current_group" "${workbench_install_dir}/workbench/.next" 2>/dev/null || true
        sudo chmod -R u+w "${workbench_install_dir}/workbench/.next" 2>/dev/null || true
    else
        log_verbose "Creating .next directory..."
        sudo mkdir -p "${workbench_install_dir}/workbench/.next" 2>/dev/null || true
        sudo chown -R "$current_user:$current_group" "${workbench_install_dir}/workbench/.next" 2>/dev/null || true
        sudo chmod -R u+w "${workbench_install_dir}/workbench/.next" 2>/dev/null || true
    fi
    
    sudo chmod u+w "${workbench_install_dir}" 2>/dev/null || true
    
    log_info "Workbench installed to ${workbench_install_dir}/workbench"
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
        log_error "Error: Binary not found at $binary_path"
        exit 1
    fi

    log_info "Installing binary to ${install_dir}..."
    if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
        cp "$binary_path" "${install_dir}/rohas.exe"
        log_info "Rohas installed to ${install_dir}/rohas.exe"
    else
        chmod +x "$binary_path"
        if [[ "$VERBOSE" == "true" ]]; then
            sudo install -m 0755 "$binary_path" "${install_dir}/rohas"
        else
            sudo install -m 0755 "$binary_path" "${install_dir}/rohas" >/dev/null 2>&1
        fi
        log_info "Rohas installed to ${install_dir}/rohas"
    fi

    log_info ""
    log_info "Done! Add ${install_dir} to your PATH if it's not already there."
    log_info "You can now run 'rohas --help'."
    
    if [[ "$INSTALL_WORKBENCH" == "true" ]]; then
        local workbench_install_dir="${ROHAS_WORKBENCH_DIR:-$DEFAULT_WORKBENCH_DIR}"
        log_info ""
        log_info "Workbench installed to ${workbench_install_dir}/workbench"
        log_info "You can now run 'rohas dev --workbench' to start the development server with workbench."
    fi
}

show_banner() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║     ██████╗  ██████╗ ██╗  ██╗ █████╗ ███████╗               ║"
    echo "║     ██╔══██╗██╔═══██╗██║  ██║██╔══██╗██╔════╝               ║"
    echo "║     ██████╔╝██║   ██║███████║███████║███████╗               ║"
    echo "║     ██╔══██╗██║   ██║██╔══██║██╔══██║╚════██║               ║"
    echo "║     ██║  ██║╚██████╔╝██║  ██║██║  ██║███████║               ║"
    echo "║     ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝               ║"
    echo "║                                                              ║"
    echo "║                    Build & Install Script                    ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
}

main() {
    show_banner

    require_cmd git
    detect_platform
    check_rust
    ensure_python
    install_build_dependencies

    # Check if we're already in the rohas repo
    local in_repo=false
    if [[ -f "Cargo.toml" ]] && grep -q "rohas" "Cargo.toml" 2>/dev/null; then
        in_repo=true
        log_verbose "Detected rohas repository in current directory."
    else
        TMP_DIR="$(mktemp -d)"
        cd "$TMP_DIR"
    fi

    clone_repo
    build_project

    if [[ "$INSTALL_WORKBENCH" == "true" ]]; then
        check_node
        build_workbench
        install_workbench
    fi

    if [[ "$INSTALL_BINARY" == "true" ]]; then
        install_binary
    else
        log_info ""
        log_info "Build completed. Binary is available at: target/${TARGET}/release/rohas"
        if [[ "$(uname -s)" == "MINGW"* ]] || [[ "$(uname -s)" == "MSYS"* ]] || [[ "$(uname -s)" == "CYGWIN"* ]]; then
            log_info "Binary location: target/${TARGET}/release/rohas.exe"
            log_info ""
            log_info "To install the binary, copy it to a directory in your PATH:"
            log_info "  cp target/${TARGET}/release/rohas.exe <path-in-PATH>/rohas.exe"
        else
            log_info ""
            log_info "To install the binary, run:"
            log_info "  sudo install -m 0755 target/${TARGET}/release/rohas ${DEFAULT_INSTALL_DIR}/rohas"
        fi
        log_info ""
        log_info "Or set ROHAS_INSTALL=true (default) and rerun this script."
    fi
}

main "$@"

