#!/bin/bash
# Sets up packaging dependencies for Linux and Windows targets.
set -e

install_cargo_tool() {
    local tool=$1
    if ! command -v "$tool" &> /dev/null; then
        echo "󰇚 Installing $tool..."
        cargo install "$tool" --quiet
    fi
}

case "$OSTYPE" in
    linux-gnu*)
        install_cargo_tool "cargo-deb"
        install_cargo_tool "cargo-generate-rpm"
    ;;
    msys*|cygwin*|win32*)
        install_cargo_tool "cargo-wix"
    ;;
esac
