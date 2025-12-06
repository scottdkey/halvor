#!/bin/bash

# Script to validate GitHub Actions workflows locally
# This checks for common issues before pushing

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Validating GitHub Actions Workflows..."
echo ""

ERRORS=0

# Check if workflow files exist
check_file() {
    if [ ! -f "$1" ]; then
        echo -e "${RED}✗ Missing: $1${NC}"
        ((ERRORS++))
        return 1
    else
        echo -e "${GREEN}✓ Found: $1${NC}"
        return 0
    fi
}

# Check if referenced files exist
check_referenced_files() {
    local workflow_file="$1"
    echo "Checking files referenced in $workflow_file..."
    
    # Extract file paths from workflow
    local files=$(grep -E "(path:|file:)" "$workflow_file" | grep -v "#" | sed 's/.*- \(.*\)/\1/' | sed 's/.*path: \(.*\)/\1/' | tr -d '"' | tr -d "'" | head -20)
    
    for file in $files; do
        # Skip GitHub Actions expressions
        if [[ "$file" == *"${{"* ]] || [[ "$file" == *"}}"* ]]; then
            continue
        fi
        
        # Check if file exists
        if [ ! -f "$file" ] && [ ! -d "$file" ]; then
            echo -e "${YELLOW}⚠ Referenced file may not exist: $file${NC}"
        fi
    done
}

# Check workflow files
echo "1. Checking workflow files..."
check_file ".github/workflows/build.yml"
check_file ".github/workflows/release.yml"
echo ""

# Check required scripts
echo "2. Checking required scripts..."
check_file "scripts/install.sh"
check_file "scripts/install.ps1"
check_file "scripts/setup-ssh-hosts.sh"
check_file "scripts/setup-ssh-keys.sh"
echo ""

# Check Docker setup (optional - workflows handle missing directory gracefully)
echo "3. Checking Docker build setup..."
if [ -d "openvpn-container" ]; then
    echo -e "${GREEN}✓ openvpn-container directory exists${NC}"
    check_file "openvpn-container/Dockerfile"
    if [ -f "openvpn-container/scripts/entrypoint.sh" ]; then
        echo -e "${GREEN}✓ entrypoint.sh exists${NC}"
    else
        echo -e "${YELLOW}⚠ entrypoint.sh not found (may be optional)${NC}"
    fi
else
    echo -e "${YELLOW}⚠ openvpn-container directory not found (Docker build will be skipped)${NC}"
    echo -e "${YELLOW}  This is OK - workflows will skip Docker build if directory doesn't exist${NC}"
fi
echo ""

# Check Cargo.toml
echo "4. Checking Rust project..."
check_file "Cargo.toml"
check_file "src/main.rs"
echo ""

# Validate YAML syntax (basic check)
echo "5. Validating YAML syntax..."
if command -v python3 &> /dev/null; then
    if python3 -c "import yaml" 2>/dev/null; then
        python3 -c "import yaml, sys; yaml.safe_load(open('.github/workflows/build.yml'))" && echo -e "${GREEN}✓ build.yml syntax valid${NC}" || { echo -e "${RED}✗ build.yml syntax error${NC}"; ((ERRORS++)); }
        python3 -c "import yaml, sys; yaml.safe_load(open('.github/workflows/release.yml'))" && echo -e "${GREEN}✓ release.yml syntax valid${NC}" || { echo -e "${RED}✗ release.yml syntax error${NC}"; ((ERRORS++)); }
    else
        echo -e "${YELLOW}⚠ Python yaml module not available, skipping YAML validation${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Python3 not available, skipping YAML validation${NC}"
fi
echo ""

# Check for common workflow issues
echo "6. Checking for common issues..."

# Check if Dockerfile path is correct (optional - workflows handle missing directory)
if grep -q "context: ./openvpn-container" ".github/workflows/build.yml" 2>/dev/null; then
    if [ ! -d "openvpn-container" ]; then
        echo -e "${YELLOW}⚠ Workflow references openvpn-container but directory doesn't exist${NC}"
        echo -e "${YELLOW}  Workflows will skip Docker build gracefully${NC}"
    else
        echo -e "${GREEN}✓ Docker context path is valid${NC}"
    fi
fi

# Check if scripts paths are correct
if grep -q "scripts/install.sh" ".github/workflows/build.yml" 2>/dev/null; then
    if [ ! -f "scripts/install.sh" ]; then
        echo -e "${RED}✗ Workflow references scripts/install.sh but file doesn't exist${NC}"
        ((ERRORS++))
    else
        echo -e "${GREEN}✓ Script paths are valid${NC}"
    fi
fi

echo ""

# Summary
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo "Workflows are ready to push. They will:"
    echo "  - Build Rust CLI for multiple platforms"
    echo "  - Build and push Docker image to GHCR"
    echo "  - Upload install scripts as artifacts"
    exit 0
else
    echo -e "${RED}✗ Found $ERRORS error(s)${NC}"
    echo "Please fix the issues above before pushing."
    exit 1
fi
