#!/usr/bin/env bash
set -e

# AirDrop Android Cross-Compilation Script
# Requires Android NDK and cargo-ndk: cargo install cargo-ndk

if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "Warning: ANDROID_NDK_HOME is not set."
    echo "Please set it, e.g.: export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/26.1.10909125"
fi

TARGETS=(
    "aarch64-linux-android:arm64-v8a"
    "armv7-linux-androideabi:armeabi-v7a"
    "x86_64-linux-android:x86_64"
)

OUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/android/app/src/main/jniLibs"

echo "=== Building airdrop-jni for Android targets ==="

for TARGET_PAIR in "${TARGETS[@]}"; do
    IFS=":" read -r RUST_TARGET ABI <<< "$TARGET_PAIR"
    echo "Building for $RUST_TARGET ($ABI)..."
    
    rustup target add "$RUST_TARGET" 2>/dev/null || true
    
    cargo ndk -t "$ABI" -o "$OUT_DIR" build --release -p airdrop-jni
    
    echo "Installed libairdrop_jni.so to $OUT_DIR/$ABI/"
done

echo "=== Android native libraries successfully compiled ==="
