#!/bin/bash
# Test script to diagnose join token issues

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Join Token Diagnostic Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This script will:"
echo "  1. Generate a token on this machine (baulder)"
echo "  2. Show where the database is located"
echo "  3. Show debug output from token generation"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check database location
echo "Database location:"
if [ -f ~/.config/halvor/halvor.db ]; then
    echo "  ✓ Database exists at: ~/.config/halvor/halvor.db"
    ls -lh ~/.config/halvor/halvor.db
else
    echo "  ✗ Database NOT found at: ~/.config/halvor/halvor.db"
fi
echo ""

# Generate token
echo "Generating token..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
halvor agent token 2>&1
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo ""
echo "Choose ONE of these options:"
echo ""
echo "OPTION A - Foreground mode (easier for debugging):"
echo "  1. Copy the token from above"
echo "  2. On baulder, run: halvor agent start"
echo "     (Keep this terminal open - all debug output will appear here)"
echo "  3. On mint, run: halvor agent join <token>"
echo "  4. Watch baulder's terminal for [AGENT SERVER] and [DEBUG] messages"
echo ""
echo "OPTION B - Daemon mode with log following:"
echo "  1. Copy the token from above"
echo "  2. On baulder, run: halvor agent start --daemon"
echo "  3. In another terminal on baulder, run: halvor agent logs -f"
echo "  4. On mint, run: halvor agent join <token>"
echo "  5. Watch the logs terminal for [AGENT SERVER] and [DEBUG] messages"
echo ""
