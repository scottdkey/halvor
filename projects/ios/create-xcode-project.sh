#!/bin/bash
set -e

# Script to create Xcode project for HalvorApp using xcodegen

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Creating Xcode project for HalvorApp..."
echo ""

# Check if xcodegen is installed
if ! command -v xcodegen >/dev/null 2>&1; then
    echo "Error: xcodegen is not installed"
    echo "Install it with: brew install xcodegen"
    exit 1
fi

# Check if project.yml exists
if [ ! -f "$SCRIPT_DIR/project.yml" ]; then
    echo "Error: project.yml not found"
    exit 1
fi

# Generate the Xcode project
cd "$SCRIPT_DIR"
xcodegen generate

echo ""
echo "✓ Xcode project created: HalvorApp.xcodeproj"
echo ""
echo "To open the project:"
echo "  open HalvorApp.xcodeproj"
echo ""
echo "Or run:"
echo "  make open-xcode"
