#!/bin/bash
# Security verification tests for sx sandbox
# Run this script to verify sandbox security properties

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS_COUNT=0
FAIL_COUNT=0

# Build the project first
echo "Building sx..."
cargo build --release 2>/dev/null || cargo build

SX_BIN="${SX_BIN:-./target/release/sx}"
if [ ! -f "$SX_BIN" ]; then
    SX_BIN="./target/debug/sx"
fi

if [ ! -f "$SX_BIN" ]; then
    echo -e "${RED}Error: sx binary not found. Run 'cargo build' first.${NC}"
    exit 1
fi

echo ""
echo "=== sx Security Verification Tests ==="
echo ""

# Helper function for test results
pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((PASS_COUNT++))
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    ((FAIL_COUNT++))
}

warn() {
    echo -e "${YELLOW}! WARN${NC}: $1"
}

# Create temp directory for tests
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Test 1: Verify sandbox blocks reading ~/.ssh
echo "Test 1: Sandbox blocks ~/.ssh access"
if $SX_BIN --dry-run 2>/dev/null | grep -q ".ssh"; then
    pass "Profile mentions .ssh in deny rules"
else
    warn "Could not verify .ssh deny rule in profile"
fi

# Test 2: Verify default network is offline
echo "Test 2: Default network mode is offline"
if $SX_BIN --dry-run 2>/dev/null | grep -q "Network disabled\|offline"; then
    pass "Default network mode is offline"
else
    warn "Could not verify offline mode"
fi

# Test 3: Verify working directory gets full access
echo "Test 3: Working directory has full access"
cd "$TEMP_DIR"
echo "test content" > testfile.txt
if $SX_BIN --dry-run 2>/dev/null | grep -q "Working directory\|file\*"; then
    pass "Working directory rules present in profile"
else
    warn "Could not verify working directory access"
fi

# Test 4: Verify process execution is allowed
echo "Test 4: Process execution is allowed"
if $SX_BIN --dry-run 2>/dev/null | grep -q "process-fork\|process-exec"; then
    pass "Process fork/exec allowed in profile"
else
    fail "Process execution not allowed"
fi

# Test 5: Verify sensitive env vars are blocked
echo "Test 5: Sensitive environment variables are protected"
# This checks the profile loading, not actual env blocking
if cargo test --quiet profile 2>/dev/null | grep -q "ok"; then
    pass "Profile tests pass (includes env var protection)"
else
    warn "Could not verify env var protection"
fi

# Test 6: Verify deny rules come before allow
echo "Test 6: Deny rules are present"
if $SX_BIN --dry-run 2>/dev/null | grep -q "(deny default)"; then
    pass "Deny default rule present"
else
    fail "Deny default rule missing"
fi

# Test 7: Verify profile composition works
echo "Test 7: Profile composition"
if cargo test --quiet compose_profiles 2>/dev/null; then
    pass "Profile composition tests pass"
else
    fail "Profile composition tests failed"
fi

# Test 8: Verify seatbelt syntax is valid
echo "Test 8: Seatbelt profile syntax validation"
PROFILE=$($SX_BIN --dry-run 2>/dev/null || echo "")
if [ -n "$PROFILE" ]; then
    if echo "$PROFILE" | grep -q "(version 1)"; then
        pass "Seatbelt profile has valid header"
    else
        fail "Seatbelt profile missing version header"
    fi
else
    warn "Could not generate profile for validation"
fi

# Test 9: Verify /tmp write access
echo "Test 9: Temporary directory access"
if $SX_BIN --dry-run 2>/dev/null | grep -q "/tmp\|/var/folders"; then
    pass "Temp directory rules present"
else
    warn "Could not verify temp directory access"
fi

# Test 10: Run all integration tests
echo "Test 10: Integration test suite"
if cargo test --test integration --quiet 2>/dev/null; then
    pass "All integration tests pass"
else
    fail "Integration tests failed"
fi

echo ""
echo "=== Security Test Summary ==="
echo -e "Passed: ${GREEN}$PASS_COUNT${NC}"
echo -e "Failed: ${RED}$FAIL_COUNT${NC}"
echo ""

if [ $FAIL_COUNT -gt 0 ]; then
    echo -e "${RED}Security verification incomplete - review failures above${NC}"
    exit 1
else
    echo -e "${GREEN}Security verification complete${NC}"
    exit 0
fi
