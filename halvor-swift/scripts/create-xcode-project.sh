#!/bin/bash
set -e

# Script to create/regenerate Xcode project from project.yml using xcodegen
# This script is called by Fastlane when the Xcode project is missing

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SWIFT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Generating Xcode project from project.yml..."

# Check if xcodegen is installed
if ! command -v xcodegen &> /dev/null; then
    echo "Error: xcodegen is not installed"
    echo "Install with: brew install xcodegen"
    exit 1
fi

# Check if project.yml exists
if [ ! -f "$SWIFT_DIR/project.yml" ]; then
    echo "Error: project.yml not found at $SWIFT_DIR/project.yml"
    exit 1
fi

# Generate Xcode project
cd "$SWIFT_DIR"
xcodegen generate

# Verify project was created
if [ ! -d "$SWIFT_DIR/HalvorApp.xcodeproj" ]; then
    echo "Error: Failed to generate Xcode project"
    exit 1
fi

echo "Successfully generated HalvorApp.xcodeproj"
