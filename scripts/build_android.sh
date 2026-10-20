#!/usr/bin/env bash
set -e

# AirDrop Android Cross-Compilation Script
# Requires Android NDK and cargo-ndk: cargo install cargo-ndk

# Auto-detect ANDROID_NDK_HOME if not explicitly set
if [ -z "$ANDROID_NDK_HOME" ]; then
    if [ -d "/opt/homebrew/share/android-commandlinetools/ndk/26.1.10909125" ]; then
        export ANDROID_NDK_HOME="/opt/homebrew/share/android-commandlinetools/ndk/26.1.10909125"
    elif [ -d "/opt/homebrew/share/android-ndk" ]; then
        export ANDROID_NDK_HOME="/opt/homebrew/share/android-ndk"
    elif compgen -G "$HOME/Library/Android/sdk/ndk/*" > /dev/null; then
        export ANDROID_NDK_HOME="$(ls -d $HOME/Library/Android/sdk/ndk/* | tail -n 1)"
    fi
fi

if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "Error: ANDROID_NDK_HOME is not set and could not be auto-detected."
    exit 1
fi

echo "Using ANDROID_NDK_HOME: $ANDROID_NDK_HOME"

REQUESTED_ABI="${1:-all}"

if [ "$REQUESTED_ABI" == "arm64-v8a" ]; then
    TARGETS=("aarch64-linux-android:arm64-v8a")
elif [ "$REQUESTED_ABI" == "all" ]; then
    TARGETS=(
        "aarch64-linux-android:arm64-v8a"
        "armv7-linux-androideabi:armeabi-v7a"
        "x86_64-linux-android:x86_64"
    )
else
    echo "Unknown ABI: $REQUESTED_ABI. Allowed: arm64-v8a, all"
    exit 1
fi

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
