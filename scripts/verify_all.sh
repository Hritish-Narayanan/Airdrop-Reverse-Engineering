#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "  Universal AirDrop: Automated Continuous Validation Suite"
echo "=========================================================="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo ""
echo "[Step 1/4] Running Rust unit & security test suite..."
cargo test --all

echo ""
echo "[Step 2/4] Verifying all workspace targets compile cleanly..."
cargo build -p airdrop-cli
cargo check --all

echo ""
echo "[Step 3/4] Testing AirDrop BLE manufacturer data generation..."
BLE_PAYLOAD=$(./target/debug/airdrop-cli ble-payload --everyone | grep "Hex Payload:" | awk '{print $3}')
if [ -z "$BLE_PAYLOAD" ]; then
    echo "❌ BLE payload generation failed!"
    exit 1
fi
echo "✓ BLE payload verified: $BLE_PAYLOAD"

echo ""
echo "[Step 4/4] Running local end-to-end AirDrop transfer test..."
TEST_DIR="$REPO_ROOT/target/test_validation"
mkdir -p "$TEST_DIR/send" "$TEST_DIR/recv"

# Create test payloads (text + binary)
echo "Universal AirDrop end-to-end automated verification test" > "$TEST_DIR/send/test_doc.txt"
dd if=/dev/urandom of="$TEST_DIR/send/test_blob.bin" bs=1024 count=20 2>/dev/null

TEST_PORT=8789

# Launch receiver in background
./target/debug/airdrop-cli receive --port $TEST_PORT --output "$TEST_DIR/recv" --name "ValidationReceiver" &
RECV_PID=$!

# Ensure receiver terminates on script exit
cleanup() {
    kill -9 $RECV_PID 2>/dev/null || true
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# Allow receiver to bind port
sleep 1

# Send files via client
./target/debug/airdrop-cli send --target "127.0.0.1:$TEST_PORT" "$TEST_DIR/send/test_doc.txt" "$TEST_DIR/send/test_blob.bin" --name "TestSender"

# Compare files byte-for-byte
cmp "$TEST_DIR/send/test_doc.txt" "$TEST_DIR/recv/test_doc.txt"
cmp "$TEST_DIR/send/test_blob.bin" "$TEST_DIR/recv/test_blob.bin"

echo "✓ Transferred files match byte-for-byte (cmp passed)!"
echo ""
echo "=========================================================="
echo "  ✅ ALL UNIVERSAL AIRDROP VALIDATION SUITES PASSED!"
echo "=========================================================="
