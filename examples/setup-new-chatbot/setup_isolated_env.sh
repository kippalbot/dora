#!/bin/bash

# Dora Voice Chat - Isolated Environment Setup
# Creates a fresh Python environment with all required Dora nodes
# Uses standardized dependency versions to avoid conflicts
# See DEPENDENCIES.md for detailed dependency specifications

set -e  # Exit on error

# Configuration
ENV_NAME="dora_voice_chat"
PYTHON_VERSION="3.12"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."  # Assumes script is in examples/setup-new-chatbot
NODE_HUB_DIR="$PROJECT_ROOT/node-hub"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Install system dependencies
install_system_dependencies() {
    print_header "Installing System Dependencies"
    
    # Install all required system dependencies
    if command -v apt-get &> /dev/null; then
        print_info "Installing system dependencies..."
        sudo apt-get update
        # Install essential build tools
        sudo apt-get install -y gcc g++ gfortran build-essential make
        # Install required libraries
        sudo apt-get install -y libopenblas-dev openssl libssl-dev
        # Install audio and multimedia libraries
        sudo apt-get install -y portaudio19-dev python3-pyaudio libgomp1 libomp-dev ffmpeg
        # Install git-lfs for large file support
        sudo apt-get install -y git-lfs
        print_success "All system dependencies installed"
    elif command -v yum &> /dev/null; then
        print_info "Installing system dependencies..."
        sudo yum install -y gcc gcc-c++ gcc-gfortran make
        sudo yum install -y openblas-devel openssl openssl-devel
        sudo yum install -y portaudio-devel libgomp-devel ffmpeg
        sudo yum install -y git-lfs
        print_success "All system dependencies installed"
    elif command -v dnf &> /dev/null; then
        print_info "Installing system dependencies..."
        sudo dnf install -y gcc gcc-c++ gcc-gfortran make
        sudo dnf install -y openblas-devel openssl openssl-devel
        sudo dnf install -y portaudio-devel libgomp-devel ffmpeg
        sudo dnf install -y git-lfs
        print_success "All system dependencies installed"
    else
        print_warning "Package manager not detected. Please install dependencies manually"
        print_info "Ubuntu/Debian: sudo apt install gcc g++ gfortran build-essential libopenblas-dev openssl libssl-dev portaudio19-dev libgomp1 libomp-dev ffmpeg git-lfs"
        print_info "RHEL/CentOS: sudo yum install gcc gcc-c++ gcc-gfortran openblas-devel openssl-devel portaudio-devel libgomp-devel ffmpeg git-lfs"
        print_info "Fedora: sudo dnf install gcc gcc-c++ gcc-gfortran openblas-devel openssl-devel portaudio-devel libgomp-devel ffmpeg git-lfs"
    fi
}

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    # Check conda
    if command -v conda &> /dev/null; then
        print_success "Conda found: $(conda --version)"
    else
        print_error "Conda not found. Please install Miniconda or Anaconda"
        echo ""
        echo "============================================"
        echo "CONDA INSTALLATION INSTRUCTIONS"
        echo "============================================"
        echo ""
        echo "Choose ONE of the following options:"
        echo ""
        echo "OPTION A: Install Miniconda (RECOMMENDED - lightweight, ~250MB)"
        echo "----------------------------------------------------------------"
        echo "# Download Miniconda installer"
        echo "wget https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh"
        echo ""
        echo "# Run the installer (press Enter for defaults, type 'yes' when asked)"
        echo "bash Miniconda3-latest-Linux-x86_64.sh"
        echo ""
        echo "# Activate conda in your current session"
        echo "source ~/.bashrc"
        echo ""
        echo "# Verify installation"
        echo "conda --version"
        echo ""
        echo "OPTION B: Install Anaconda (Full distribution, ~3GB)"
        echo "----------------------------------------------------"
        echo "# Download Anaconda installer"
        echo "wget https://repo.anaconda.com/archive/Anaconda3-2024.06-1-Linux-x86_64.sh"
        echo ""
        echo "# Run the installer (press Enter for defaults, type 'yes' when asked)"
        echo "bash Anaconda3-2024.06-1-Linux-x86_64.sh"
        echo ""
        echo "# Activate conda in your current session"
        echo "source ~/.bashrc"
        echo ""
        echo "# Verify installation"
        echo "conda --version"
        echo ""
        echo "After installation, run this script again."
        echo "============================================"
        exit 1
    fi
    
    # Check git
    if command -v git &> /dev/null; then
        print_success "Git found: $(git --version)"
    else
        print_error "Git not found. Please install git"
        exit 1
    fi
    
    # Check cargo (optional, for Rust nodes)
    if command -v cargo &> /dev/null; then
        print_success "Cargo found: $(cargo --version)"
    else
        print_warning "Cargo not found. Rust nodes will not be built"
        print_info "Install from: https://rustup.rs/"
    fi
}

# Create conda environment
create_environment() {
    print_header "Creating Conda Environment: $ENV_NAME"
    
    # Check if environment already exists
    if conda env list | grep -q "^$ENV_NAME "; then
        print_warning "Environment '$ENV_NAME' already exists"
        read -p "Do you want to remove and recreate it? (y/n): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Removing existing environment..."
            conda env remove -n $ENV_NAME -y
        else
            print_info "Using existing environment"
            return
        fi
    fi
    
    print_info "Creating new conda environment with Python $PYTHON_VERSION..."
    conda create -n $ENV_NAME python=$PYTHON_VERSION -y
    print_success "Environment created successfully"
}

# Setup conda in shell profile
setup_conda_profile() {
    print_header "Setting up Conda in Shell Profile"
    
    CONDA_BASE_PATH=$(conda info --base 2>/dev/null || echo "")
    if [ -n "$CONDA_BASE_PATH" ]; then
        print_info "Adding conda to shell profile..."
        
        # Add to bashrc if not already there
        if ! grep -q "conda.sh" ~/.bashrc 2>/dev/null; then
            echo "" >> ~/.bashrc
            echo "# Initialize conda" >> ~/.bashrc
            echo "if [ -f \"$CONDA_BASE_PATH/etc/profile.d/conda.sh\" ]; then" >> ~/.bashrc
            echo "    source \"$CONDA_BASE_PATH/etc/profile.d/conda.sh\"" >> ~/.bashrc
            echo "fi" >> ~/.bashrc
            echo "export PATH=\"$CONDA_BASE_PATH/bin:\$PATH\"" >> ~/.bashrc
            echo "export PATH=\"$CONDA_BASE_PATH/condabin:\$PATH\"" >> ~/.bashrc
            print_success "Conda added to ~/.bashrc"
        else
            print_info "Conda already configured in ~/.bashrc"
        fi
        
        # Add to zshrc if it exists
        if [ -f ~/.zshrc ] && ! grep -q "conda.sh" ~/.zshrc 2>/dev/null; then
            echo "" >> ~/.zshrc
            echo "# Initialize conda" >> ~/.zshrc
            echo "if [ -f \"$CONDA_BASE_PATH/etc/profile.d/conda.sh\" ]; then" >> ~/.zshrc
            echo "    source \"$CONDA_BASE_PATH/etc/profile.d/conda.sh\"" >> ~/.zshrc
            echo "fi" >> ~/.zshrc
            echo "export PATH=\"$CONDA_BASE_PATH/bin:\$PATH\"" >> ~/.zshrc
            echo "export PATH=\"$CONDA_BASE_PATH/condabin:\$PATH\"" >> ~/.zshrc
            print_success "Conda added to ~/.zshrc"
        fi
        
        # Add to system-wide profile if we have sudo access
        if ! grep -q "$CONDA_BASE_PATH" /etc/profile 2>/dev/null; then
            if sudo -n true 2>/dev/null; then
                print_info "Adding conda to system-wide profile..."
                echo "export PATH=\"$CONDA_BASE_PATH/bin:$CONDA_BASE_PATH/condabin:\$PATH\"" | sudo tee -a /etc/profile > /dev/null
                print_success "Conda added to /etc/profile"
            else
                print_warning "Cannot add conda to system profile (no sudo access)"
                print_info "You may need to restart your terminal or run: source ~/.bashrc"
            fi
        fi
    else
        print_warning "Could not detect conda base path"
    fi
}

# Activate environment and install dependencies
install_dependencies() {
    print_header "Installing Dependencies"
    
    # Activate environment
    eval "$(conda shell.bash hook)"
    conda activate $ENV_NAME
    
    print_info "Active Python: $(which python)"
    print_info "Python version: $(python --version)"
    
    # Upgrade pip
    print_info "Upgrading pip..."
    pip install --upgrade pip
    
    # Install critical dependencies with specific versions
    print_info "Installing core dependencies..."
    # Install standardized versions (see DEPENDENCIES.md)
    pip install numpy==1.26.4  # Voice chat pipeline standard (1.x compatibility)
    pip install torch==2.2.0 torchvision==0.17.0 torchaudio==2.2.0 --index-url https://download.pytorch.org/whl/cpu
    
    # Install transformers and related packages
    print_info "Installing ML libraries..."
    pip install transformers==4.45.0  # Voice chat pipeline standard (security compliant)
    pip install huggingface-hub==0.34.4
    pip install datasets accelerate sentencepiece protobuf
    
    # Install dora-rs
    print_info "Installing dora-rs..."
    pip install dora-rs==0.3.12
    
    # Install other dependencies
    print_info "Installing additional dependencies..."
    pip install pyarrow scipy librosa soundfile webrtcvad
    pip install openai websockets aiohttp requests
    pip install pyyaml toml python-dotenv
    pip install pyaudio sounddevice
    
    # Install llama-cpp-python from conda-forge (avoids build issues)
    print_info "Installing llama-cpp-python from conda-forge..."
    conda install -c conda-forge llama-cpp-python -y

    # Install TTS backends
    print_info "Installing TTS backends..."
    pip install kokoro  # CPU backend (cross-platform)

    # Install MLX backend (macOS only - Apple Silicon GPU acceleration)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        print_info "Installing MLX audio backend (Apple Silicon GPU acceleration)..."
        pip install mlx-audio
        print_success "MLX audio backend installed (GPU-accelerated TTS)"
    else
        print_warning "Skipping MLX audio backend (macOS only)"
        print_info "Using CPU backend for TTS (cross-platform compatible)"
    fi

    print_success "Core dependencies installed"
}

# Install and check dora CLI
install_dora_cli() {
    print_header "Installing Dora CLI"
    
    # Check if cargo is available
    if command -v cargo &> /dev/null; then
        print_info "Installing dora-cli via cargo..."
        cargo install dora-cli --locked
        
        # Check if installation was successful
        if [ -f "$HOME/.cargo/bin/dora" ]; then
            VERSION=$($HOME/.cargo/bin/dora --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
            if [ "$VERSION" = "0.3.12" ]; then
                # Link to conda environment
                ln -sf "$HOME/.cargo/bin/dora" "$CONDA_PREFIX/bin/dora"
                print_success "Dora CLI version 0.3.12 installed and linked to environment"
            else
                print_warning "Dora CLI installed but version is $VERSION (expected 0.3.12)"
            fi
        else
            print_warning "Dora CLI installation failed"
        fi
    else
        print_warning "Cargo not found. Cannot install dora-cli via cargo."
        print_info "Install Rust from https://rustup.rs/ to get the latest dora-cli"
        print_info "Using dora from pip installation instead"
    fi
}

# Install Dora nodes
install_dora_nodes() {
    print_header "Installing Dora Nodes"
    
    # List of Python nodes to install
    NODES=(
        "dora-asr"
        "dora-primespeech"
        "dora-kokoro-tts"
        "dora-qwen3"
        "dora-text-segmenter"
        "dora-speechmonitor"
    )
    
    for node in "${NODES[@]}"; do
        NODE_PATH="$NODE_HUB_DIR/$node"
        if [ -d "$NODE_PATH" ]; then
            print_info "Installing $node..."
            pip install -e "$NODE_PATH"
            print_success "$node installed"
        else
            print_warning "$node not found at $NODE_PATH"
        fi
    done
    
    # Build Rust nodes if cargo is available
    if command -v cargo &> /dev/null; then
        print_info "Building Rust nodes..."
        
        # Build dora-maas-client
        if [ -d "$NODE_HUB_DIR/dora-maas-client" ]; then
            print_info "Building dora-maas-client..."
            cd "$NODE_HUB_DIR/dora-maas-client"
            cargo build --release
            print_success "dora-maas-client built"
        fi
        
        # Build dora-openai-websocket
        if [ -d "$NODE_HUB_DIR/dora-openai-websocket" ]; then
            print_info "Building dora-openai-websocket..."
            cd "$NODE_HUB_DIR/dora-openai-websocket"
            cargo build --release -p dora-openai-websocket
            print_success "dora-openai-websocket built"
        fi
        
        cd "$SCRIPT_DIR"
    else
        print_warning "Skipping Rust node builds (cargo not found)"
    fi
}

# Fix numpy compatibility
fix_numpy_compatibility() {
    print_header "Fixing NumPy Compatibility"
    
    print_info "Ensuring numpy 1.26.4 is installed..."
    pip install numpy==1.26.4 --force-reinstall  # Ensure 1.x compatibility
    
    print_success "NumPy compatibility fixed"
}

# Run tests
run_tests() {
    print_header "Running Node Tests"
    
    if [ -d "$SCRIPT_DIR/tests" ]; then
        print_info "Running test suite..."
        python "$SCRIPT_DIR/tests/run_all_tests.py"
    else
        print_warning "Test directory not found"
    fi
}

# Print summary
print_summary() {
    print_header "Setup Complete!"

    echo ""
    echo "Environment Name: $ENV_NAME"
    echo "Python Version: $PYTHON_VERSION"
    echo ""

    # Check which TTS backends are available
    echo "TTS Backends Installed:"
    echo "  ✓ CPU (kokoro) - Cross-platform, best for short text (<150 chars)"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "  ✓ MLX (mlx-audio) - Apple Silicon GPU, best for long text (>200 chars)"
        echo ""
        echo "TTS Backend Selection:"
        echo "  Set BACKEND=cpu   for CPU backend (1.8x faster for short text)"
        echo "  Set BACKEND=mlx   for MLX backend (up to 3x faster for long text)"
        echo "  Set BACKEND=auto  to auto-detect (default)"
    fi
    echo ""

    echo "To activate the environment:"
    echo "  conda activate $ENV_NAME"
    echo ""
    echo "To test the installation:"
    echo "  cd $SCRIPT_DIR"
    echo "  python tests/run_all_tests.py"
    echo ""
    echo "To run examples:"
    echo "  cd $PROJECT_ROOT/examples/mac-aec-chat"
    echo "  dora up"
    echo "  dora start voice-chat-with-aec.yml"
    echo ""
    echo "To test Kokoro TTS backends:"
    echo "  cd $SCRIPT_DIR/kokoro-tts-validation"
    echo "  ./run_all_tests.sh"
    echo ""
    print_success "Setup completed successfully!"
}

# Main execution
main() {
    print_header "Dora Voice Chat - Isolated Environment Setup"
    
    check_prerequisites
    install_system_dependencies
    setup_conda_profile
    create_environment
    
    # Activate environment for remaining steps
    eval "$(conda shell.bash hook)"
    conda activate $ENV_NAME
    
    install_dependencies
    install_dora_cli
    install_dora_nodes
    fix_numpy_compatibility
    
    print_summary
}

# Run main function
main "$@"