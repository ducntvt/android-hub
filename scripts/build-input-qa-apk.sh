#!/bin/bash
# Build the offline, permission-free Android input fixture into a new directory.
set -euo pipefail
out="${1:?Pass a new output directory}"
[[ ! -e "$out" ]] || { echo 'Output already exists' >&2; exit 1; }
mkdir -p "$out/classes" "$out/dex"
out="$(cd "$out" && pwd)"
root="$(cd "$(dirname "$0")/.." && pwd)"
fixture="${2:-$root/tests/android-input}"
fixture="$(cd "$fixture" && pwd)"
sdk="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}"
build="$sdk/build-tools/35.0.1"
java="${ANDROID_HUB_JAVA_HOME:-${P0_JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}}"
"$java/bin/javac" --release 8 -classpath "$sdk/platforms/android-35/android.jar" -d "$out/classes" "$fixture/InputActivity.java"
JAVA_HOME="$java" "$build/d8" --release --min-api 31 --lib "$sdk/platforms/android-35/android.jar" --output "$out/dex" "$out/classes/local/androidhub/inputqa/"*.class
"$build/aapt2" link -o "$out/unsigned.apk" --manifest "$fixture/AndroidManifest.xml" -I "$sdk/platforms/android-35/android.jar"
(cd "$out/dex" && /usr/bin/zip -q "$out/unsigned.apk" classes.dex)
"$build/zipalign" 4 "$out/unsigned.apk" "$out/aligned.apk"
"$java/bin/keytool" -genkeypair -keystore "$out/test.jks" -storepass testonly -keypass testonly -alias smoke -keyalg RSA -validity 2 -dname 'CN=Local Hub Input QA'
JAVA_HOME="$java" "$build/apksigner" sign --ks "$out/test.jks" --ks-pass pass:testonly --key-pass pass:testonly --out "$out/input-qa.apk" "$out/aligned.apk"
echo "$out/input-qa.apk"
