#!/bin/bash
# Security verification tests for sx sandbox
#
# Two layers:
#   A. Policy shape  - the generated policy says what we expect (platform-specific)
#   B. Behaviour     - the sandbox actually denies what it claims (platform-neutral)
#
# Layer B is the one that matters: a sandbox that silently fails to sandbox is
# worse than no sandbox at all.

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS_COUNT=0
FAIL_COUNT=0

PLATFORM=$(uname -s)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "Building sx..."
cargo build --release 2>/dev/null || cargo build

SX_BIN="${SX_BIN:-$REPO_ROOT/target/release/sx}"
if [ ! -x "$SX_BIN" ]; then
    SX_BIN="$REPO_ROOT/target/debug/sx"
fi

if [ ! -x "$SX_BIN" ]; then
    echo -e "${RED}Error: sx binary not found. Run 'cargo build' first.${NC}"
    exit 1
fi

echo ""
echo "=== sx Security Verification Tests ($PLATFORM) ==="
echo ""

pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

warn() {
    echo -e "${YELLOW}! WARN${NC}: $1"
}

skip() {
    echo -e "${YELLOW}- SKIP${NC}: $1"
}

# Workspace outside /tmp: the base profile grants /tmp on purpose, so a
# directory under $HOME is what "outside the sandbox" actually looks like.
WORK_ROOT=$(mktemp -d "$HOME/.sx-security-test.XXXXXX")
trap 'rm -rf "$WORK_ROOT"' EXIT
mkdir -p "$WORK_ROOT/work" "$WORK_ROOT/outside" "$WORK_ROOT/vault"
echo "TOPSECRET" > "$WORK_ROOT/outside/secret.txt"
echo "TOPSECRET" > "$WORK_ROOT/vault/key"
WORK="$WORK_ROOT/work"

# Run sx with the working directory set to the sandbox workspace.
sx_in_work() {
    (cd "$WORK" && "$SX_BIN" "$@")
}

POLICY=$(cd "$WORK" && "$SX_BIN" --dry-run 2>/dev/null || echo "")

# Which mechanisms this machine actually uses, so a green run says what it
# covered. On Linux the network path depends on whether user namespaces work.
echo "$POLICY" | grep -E "^# (kernel|network|setuid)" | sed 's/^# /  /'
echo ""

echo "--- A. Policy shape ---"

# A1: sensitive home paths are denied
echo "Test A1: Sensitive paths appear in the deny rules"
SECRET_DIR=".ssh"
if echo "$POLICY" | grep -q "$SECRET_DIR"; then
    pass "Policy denies $SECRET_DIR"
else
    fail "Policy does not mention $SECRET_DIR"
fi

# A2: default network mode is offline
echo "Test A2: Default network mode is offline"
if echo "$POLICY" | grep -qi "Network disabled\|network: offline"; then
    pass "Default network mode is offline"
else
    fail "Default network mode is not offline"
fi

# A3: working directory gets full access
echo "Test A3: Working directory has full access"
case "$PLATFORM" in
    Darwin) WD_PATTERN="Working directory\|file\*" ;;
    *)      WD_PATTERN="^rlw " ;;
esac
if echo "$POLICY" | grep -q "$WD_PATTERN"; then
    pass "Working directory rules present in policy"
else
    fail "Working directory rules missing from policy"
fi

# A4: deny-by-default model is in force
echo "Test A4: Deny-by-default model"
case "$PLATFORM" in
    Darwin) DEFAULT_PATTERN="(deny default)" ;;
    *)      DEFAULT_PATTERN="sx sandbox policy (landlock)" ;;
esac
if echo "$POLICY" | grep -qF "$DEFAULT_PATTERN"; then
    pass "Deny-by-default policy header present"
else
    fail "Deny-by-default policy header missing"
fi

# A5: temp directory is usable
echo "Test A5: Temporary directory access"
if echo "$POLICY" | grep -q "/tmp\|/var/folders"; then
    pass "Temp directory rules present"
else
    fail "Temp directory rules missing"
fi

# A6: platform-specific policy validity
echo "Test A6: Policy is well-formed for this platform"
case "$PLATFORM" in
    Darwin)
        if echo "$POLICY" | grep -qF "(version 1)"; then
            pass "Seatbelt profile has a valid header"
        else
            fail "Seatbelt profile missing version header"
        fi
        ;;
    Linux)
        if echo "$POLICY" | grep -q "kernel Landlock ABI: [1-9]"; then
            pass "Landlock is supported and reported by the kernel"
        else
            fail "Kernel does not report Landlock support"
        fi
        ;;
esac

echo ""
echo "--- B. Enforcement behaviour ---"

# Some macOS images (including GitHub-hosted runners) refuse custom
# deny-default Seatbelt profiles, so probe once before asserting behaviour.
# On Linux there is no such restriction: a failure here is a real failure.
if sx_in_work -- /bin/echo probe >/dev/null 2>&1; then
    SANDBOX_RUNS=1
else
    SANDBOX_RUNS=0
fi

if [ "$SANDBOX_RUNS" = "0" ]; then
    if [ "$PLATFORM" = "Darwin" ]; then
        skip "Custom Seatbelt profiles are unavailable on this system - behaviour tests skipped"
    else
        fail "The sandbox could not run a command at all"
    fi
fi

run_behaviour_tests() {

# B1: the sandbox runs commands at all
echo "Test B1: Commands execute inside the sandbox"
if sx_in_work -- /bin/echo sandboxed >/dev/null 2>&1; then
    pass "Sandboxed command executed"
else
    fail "Sandboxed command failed to execute"
fi

# B2: the working directory is writable
echo "Test B2: Working directory is writable"
if sx_in_work -- /usr/bin/touch "$WORK/created.txt" >/dev/null 2>&1 && [ -f "$WORK/created.txt" ]; then
    pass "Working directory is writable"
else
    fail "Working directory is not writable"
fi

# B3: home directory is not readable by default
echo "Test B3: Home directory is not readable by default"
if sx_in_work -- /bin/ls "$HOME" >/dev/null 2>&1; then
    fail "Home directory was listable (deny-by-default is not in force)"
else
    pass "Home directory is not listable"
fi

# B4: files outside the working directory are not readable
echo "Test B4: Files outside the sandbox are unreadable"
LEAK=$(sx_in_work -- /bin/cat "$WORK_ROOT/outside/secret.txt" 2>/dev/null || true)
if [ -z "$LEAK" ]; then
    pass "File outside the sandbox stayed unreadable"
else
    fail "Leaked file contents from outside the sandbox"
fi

# B5: writes outside the working directory are blocked
echo "Test B5: Writes outside the sandbox are blocked"
sx_in_work -- /usr/bin/touch "$WORK_ROOT/outside/planted.txt" >/dev/null 2>&1 || true
if [ -f "$WORK_ROOT/outside/planted.txt" ]; then
    fail "Wrote a file outside the sandbox"
else
    pass "Write outside the sandbox was blocked"
fi

# B6: deny_read overrides an explicit allow_read
echo "Test B6: deny_read overrides allow_read"
LEAK=$(sx_in_work --allow-read "$WORK_ROOT/vault" --deny-read "$WORK_ROOT/vault" \
        -- /bin/cat "$WORK_ROOT/vault/key" 2>/dev/null || true)
if [ -z "$LEAK" ]; then
    pass "deny_read took precedence over allow_read"
else
    fail "deny_read did not override allow_read"
fi

# B7: restrictions survive fork and exec
echo "Test B7: Restrictions are inherited by child processes"
LEAK=$(sx_in_work -- /bin/sh -c "cat '$WORK_ROOT/outside/secret.txt'" 2>/dev/null || true)
if [ -z "$LEAK" ]; then
    pass "Child process inherited the sandbox"
else
    fail "Child process escaped the sandbox"
fi

# B8: offline mode blocks the network
echo "Test B8: Offline mode blocks network access"
if sx_in_work -- /usr/bin/curl -sS -m 5 -o /dev/null https://example.com >/dev/null 2>&1; then
    fail "Offline mode allowed a network connection"
else
    pass "Offline mode blocked the network"
fi

# B9: online mode still works (needs connectivity)
echo "Test B9: Online mode allows network access"
if ! curl -sS -m 5 -o /dev/null https://example.com >/dev/null 2>&1; then
    skip "No outbound connectivity on this host"
elif sx_in_work online -- /usr/bin/curl -sS -m 10 -o /dev/null https://example.com >/dev/null 2>&1; then
    pass "Online mode reached the network"
else
    fail "Online mode could not reach the network"
fi

# B10: Linux never lets a setuid binary elevate
if [ "$PLATFORM" = "Linux" ]; then
    echo "Test B10: no_new_privs is set"
    if sx_in_work -- /usr/bin/grep -q "NoNewPrivs:.1" /proc/self/status >/dev/null 2>&1; then
        pass "no_new_privs is set inside the sandbox"
    else
        fail "no_new_privs is not set"
    fi
fi

}

if [ "$SANDBOX_RUNS" = "1" ]; then
    run_behaviour_tests
fi

echo ""
echo "--- C. Test suites ---"

# CI runs the suite in its own job; skip the duplicate there.
if [ -n "$SX_SKIP_TEST_SUITE" ]; then
    skip "Test suite (run separately)"
else
    echo "Test C1: Unit and integration test suite"
    if cargo test --quiet >/dev/null 2>&1; then
        pass "All tests pass"
    else
        fail "Test suite failed"
    fi
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
