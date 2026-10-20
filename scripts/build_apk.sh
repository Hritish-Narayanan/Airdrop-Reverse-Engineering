#!/usr/bin/env bash
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Configure JDK 17 and Android SDK/NDK
if [ -d "/Library/Java/JavaVirtualMachines/jdk-17.0.3.jdk/Contents/Home" ]; then
    export JAVA_HOME="/Library/Java/JavaVirtualMachines/jdk-17.0.3.jdk/Contents/Home"
fi

if [ -z "$ANDROID_HOME" ] && [ -d "/opt/homebrew/share/android-commandlinetools" ]; then
    export ANDROID_HOME="/opt/homebrew/share/android-commandlinetools"
fi

if [ -z "$ANDROID_NDK_HOME" ] && [ -d "/opt/homebrew/share/android-commandlinetools/ndk/26.1.10909125" ]; then
    export ANDROID_NDK_HOME="/opt/homebrew/share/android-commandlinetools/ndk/26.1.10909125"
fi

echo "=== Building Universal AirDrop Android APK ==="
echo "JAVA_HOME: $JAVA_HOME"
echo "ANDROID_HOME: $ANDROID_HOME"

# Step 1: Ensure native JNI library for arm64-v8a is compiled
"$REPO_ROOT/scripts/build_android.sh" arm64-v8a

# Step 2: Build APK via Gradle
cd "$REPO_ROOT/android"
./gradlew assembleDebug

echo ""
echo "=== APK Built Successfully! ==="
echo "Path: $REPO_ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
