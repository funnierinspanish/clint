#!/bin/bash
# CLINT Installation Script
# Downloads and installs the latest release of CLINT CLI tool

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="funnierinspanish/clint"
BINARY_NAME="clint"
INSTALL_DIR="${HOME}/.local/bin"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect platform and architecture
detect_platform() {
    local os=""
    local arch=""
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="unknown-linux-gnu" ;;
        Darwin*)    os="apple-darwin" ;;
        CYGWIN*|MINGW*|MSYS*) os="pc-windows-msvc" ;;
        *)          
            log_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        arm64|aarch64)  arch="aarch64" ;;
        *)              
            log_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    # Special handling for macOS M1/M2
    if [[ "$os" == "apple-darwin" && "$arch" == "aarch64" ]]; then
        PLATFORM_TARGET="aarch64-apple-darwin"
    else
        PLATFORM_TARGET="${arch}-${os}"
    fi
    
    # Add .exe extension for Windows
    if [[ "$os" == "pc-windows-msvc" ]]; then
        BINARY_EXTENSION=".exe"
    else
        BINARY_EXTENSION=""
    fi
    
    log_info "Detected platform: $PLATFORM_TARGET"
}

# Check if required tools are available
check_dependencies() {
    local missing_deps=()
    
    for cmd in curl; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        log_info "Please install curl using your package manager:"
        log_info "  Ubuntu/Debian: sudo apt-get install curl"
        log_info "  CentOS/RHEL:   sudo yum install curl"
        log_info "  macOS:         brew install curl (or use system curl)"
        exit 1
    fi
}

# Get latest release information from GitHub
get_latest_release() {
    log_info "Fetching latest release information..."
    
    local api_url="https://api.github.com/repos/$REPO/releases/latest"
    local release_info
    
    if ! release_info=$(curl -s "$api_url"); then
        log_error "Failed to fetch release information from GitHub API"
        exit 1
    fi
    
    # Extract tag name and release assets
    LATEST_VERSION=$(echo "$release_info" | grep '"tag_name"' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
    
    if [[ -z "$LATEST_VERSION" ]]; then
        log_error "Could not determine latest version"
        exit 1
    fi
    
    log_info "Latest version: $LATEST_VERSION"
    
    # Find the correct binary for our platform
    BINARY_FILENAME="${BINARY_NAME}-${LATEST_VERSION}-${PLATFORM_TARGET}${BINARY_EXTENSION}"
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_VERSION/$BINARY_FILENAME"
    
    log_info "Binary filename: $BINARY_FILENAME"
}

# Verify the download URL exists
verify_download_url() {
    log_info "Verifying download URL..."
    
    if ! curl --output /dev/null --silent --head --fail "$DOWNLOAD_URL"; then
        log_error "Binary not available for platform $PLATFORM_TARGET"
        log_info "Available releases at: https://github.com/$REPO/releases/latest"
        exit 1
    fi
}

# Download and install the binary
download_and_install() {
    local temp_dir
    temp_dir=$(mktemp -d)
    local temp_binary="$temp_dir/$BINARY_NAME$BINARY_EXTENSION"
    
    log_info "Downloading $BINARY_FILENAME..."
    
    if ! curl -L -o "$temp_binary" "$DOWNLOAD_URL"; then
        log_error "Failed to download binary"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Create install directory if it doesn't exist
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_info "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Install the binary
    local install_path="$INSTALL_DIR/$BINARY_NAME$BINARY_EXTENSION"
    
    log_info "Installing to $install_path..."
    
    if ! mv "$temp_binary" "$install_path"; then
        log_error "Failed to install binary"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Make executable (not needed on Windows)
    if [[ "$BINARY_EXTENSION" != ".exe" ]]; then
        chmod +x "$install_path"
    fi
    
    # Cleanup
    rm -rf "$temp_dir"
    
    log_success "CLINT installed successfully to $install_path"
}

# Check if install directory is in PATH and provide shell-specific instructions
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        log_warning "$INSTALL_DIR is not in your PATH"
        echo
        log_info "To use CLINT from anywhere, add it to your PATH:"
        echo
        
        # Detect current shell and provide specific instructions
        local current_shell=""
        if [[ -n "$BASH_VERSION" ]]; then
            current_shell="bash"
        elif [[ -n "$ZSH_VERSION" ]]; then
            current_shell="zsh"
        elif [[ "$SHELL" == *"fish"* ]]; then
            current_shell="fish"
        else
            # Fallback: try to detect from SHELL environment variable
            case "$SHELL" in
                */bash) current_shell="bash" ;;
                */zsh)  current_shell="zsh" ;;
                */fish) current_shell="fish" ;;
                *)      current_shell="unknown" ;;
            esac
        fi
        
        # Provide shell-specific commands
        case "$current_shell" in
            bash)
                echo "📋 For Bash (copy and paste this command):"
                echo "   echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc && source ~/.bashrc"
                echo
                ;;
            zsh)
                echo "📋 For Zsh (copy and paste this command):"
                echo "   echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.zshrc && source ~/.zshrc"
                echo
                ;;
            fish)
                echo "📋 For Fish (copy and paste this command):"
                echo "   echo 'set -gx PATH \$PATH $INSTALL_DIR' >> ~/.config/fish/config.fish && source ~/.config/fish/config.fish"
                echo
                ;;
            *)
                echo "📋 For your shell, add this line to your shell profile:"
                echo "   export PATH=\"\$PATH:$INSTALL_DIR\""
                echo
                echo "Common profile files:"
                echo "   • Bash: ~/.bashrc or ~/.bash_profile"
                echo "   • Zsh: ~/.zshrc"
                echo "   • Fish: ~/.config/fish/config.fish"
                echo
                ;;
        esac
        
        # Offer to do it automatically (but ask for permission)
        if [[ "$current_shell" != "unknown" && "$current_shell" != "fish" ]]; then
            echo
            read -p "Would you like me to add CLINT to your PATH automatically? [y/N]: " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                case "$current_shell" in
                    bash)
                        if echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> ~/.bashrc; then
                            log_success "Added to ~/.bashrc"
                            log_info "Run 'source ~/.bashrc' or restart your terminal to use CLINT"
                            # Update current session PATH
                            export PATH="$PATH:$INSTALL_DIR"
                        else
                            log_error "Failed to update ~/.bashrc"
                        fi
                        ;;
                    zsh)
                        if echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> ~/.zshrc; then
                            log_success "Added to ~/.zshrc"
                            log_info "Run 'source ~/.zshrc' or restart your terminal to use CLINT"
                            # Update current session PATH
                            export PATH="$PATH:$INSTALL_DIR"
                        else
                            log_error "Failed to update ~/.zshrc"
                        fi
                        ;;
                esac
            else
                log_info "Skipped automatic PATH update"
                echo "💡 Tip: Copy and paste the command above to add CLINT to your PATH"
            fi
        else
            echo "💡 Tip: Copy & paste the appropriate command above, then restart your terminal"
        fi
    else
        log_success "$INSTALL_DIR is already in your PATH"
    fi
}

# Verify installation
verify_installation() {
    local install_path="$INSTALL_DIR/$BINARY_NAME$BINARY_EXTENSION"
    
    if [[ -x "$install_path" ]]; then
        log_success "Installation verified!"
        
        # Test if it's in PATH and executable
        if command -v "$BINARY_NAME" &> /dev/null; then
            local installed_version
            installed_version=$("$BINARY_NAME" --version 2>/dev/null | head -n1 || echo "unknown")
            log_success "CLINT is ready to use! Version: $installed_version"
            log_info "Try running: $BINARY_NAME --help"
        else
            log_warning "CLINT installed but not in PATH. Use full path: $install_path"
        fi
    else
        log_error "Installation verification failed"
        exit 1
    fi
}

# Handle installation for different package managers (optional)
offer_package_manager_install() {
    log_info "Alternative installation methods:"
    echo
    echo "  Homebrew (macOS):   brew install $REPO"
    echo "  Cargo (Rust):       cargo install $BINARY_NAME"
    echo "  Manual build:       git clone https://github.com/$REPO && cd clint && cargo build --release"
    echo
}

# Main installation flow
main() {
    echo "CLINT Installation Script"
    echo "========================"
    echo
    
    # Check if already installed
    if command -v "$BINARY_NAME" &> /dev/null; then
        local current_version
        current_version=$("$BINARY_NAME" --version 2>/dev/null | head -n1 || echo "unknown")
        log_info "CLINT is already installed: $current_version"
        
        read -p "Do you want to update to the latest version? [y/N]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Installation cancelled"
            exit 0
        fi
    fi
    
    # Detect platform
    detect_platform
    
    # Check dependencies
    check_dependencies
    
    # Get latest release
    get_latest_release
    
    # Verify download URL
    verify_download_url
    
    # Download and install
    download_and_install
    
    # Check PATH
    check_path
    
    # Verify installation
    verify_installation
    
    echo
    log_success "Installation complete!"
    offer_package_manager_install
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "CLINT Installation Script"
        echo
        echo "Usage: $0 [options]"
        echo
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --version, -v  Show version information"
        echo
        echo "Environment Variables:"
        echo "  INSTALL_DIR    Installation directory (default: ~/.local/bin)"
        echo
        echo "Examples:"
        echo "  $0                           # Install to ~/.local/bin"
        echo "  INSTALL_DIR=/usr/local/bin $0 # Install to /usr/local/bin"
        exit 0
        ;;
    --version|-v)
        echo "CLINT Installation Script v1.0"
        exit 0
        ;;
    "")
        # No arguments - proceed with installation
        main
        ;;
    *)
        log_error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac