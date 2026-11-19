#!/bin/bash
# Compile the repository-owned guest probe using the installed official Android SDK.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
sdk="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}"
java="${ANDROID_HUB_JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}"
out="$(mktemp -d "${TMPDIR:-/tmp}/hub-connectivity-build.XXXXXX")"
trap 'rm -rf "$out"' EXIT
mkdir -p "$out/classes" "$out/dex"
"$java/bin/javac" --release 8 -Xlint:-options -classpath "$sdk/platforms/android-35/android.jar" -d "$out/classes" "$root/crates/infrastructure/android/ConnectivityProbe.java" "$root/tests/android-connectivity/ConnectivityProbeTest.java"
"$java/bin/java" -cp "$out/classes" local.androidhub.ConnectivityProbeTest
JAVA_HOME="$java" "$sdk/build-tools/35.0.1/d8" --release --min-api 31 --lib "$sdk/platforms/android-35/android.jar" --output "$out/dex" "$out/classes/local/androidhub/ConnectivityProbe.class"
cp "$out/dex/classes.dex" "$root/crates/infrastructure/android/connectivity-probe.dex"
echo 'Built connectivity-probe.dex; helper tests passed'
