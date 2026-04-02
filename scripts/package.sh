#!/bin/bash
set -e

TARGET=$1
PKG_NAME=$2
BIN_NAME=$3
OS_TYPE=$4

mkdir -p artifacts/

if [[ "$OS_TYPE" == "ubuntu-latest" ]]; then
    echo "📦 Packaging for Debian/RPM..."
    cargo deb --target "$TARGET" --no-build
    cargo generate-rpm --target "$TARGET"
    cp target/"$TARGET"/debian/*.deb artifacts/ 2>/dev/null || true
    cp target/generate-rpm/*.rpm artifacts/ 2>/dev/null || cp target/"$TARGET"/generate-rpm/*.rpm artifacts/ 2>/dev/null || true
fi

if [[ "$OS_TYPE" == "windows-latest" ]]; then
    echo "📦 Preparing Windows binaries for MSI..."
    mkdir -p target/"$TARGET"/release
    
    cp artifacts/*.exe target/"$TARGET"/release/ 2>/dev/null || true
    
    echo "📦 Running WiX..."
    cargo wix --target "$TARGET" --no-build -o artifacts/
fi