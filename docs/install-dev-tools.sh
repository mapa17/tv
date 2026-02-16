#!/bin/bash
set -e

echo "Installing cargo dev tools..."

# Install system dependencies for cross-platform builds
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo apt-get update
    sudo apt-get install -y build-essential musl-dev mingetty mingetty-dev
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

cargo install --locked cargo-audit --version 0.22.1 --features=fix

mkdir -p .husky/hooks
cp docs/pre-commit.sh .husky/hooks/pre-commit


echo "Done."