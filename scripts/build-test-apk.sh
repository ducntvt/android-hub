#!/bin/bash
# Minimal local package for APK installation smoke tests; contains no executable code.
set -euo pipefail
out="${1:?Pass a new output directory}"
[[ ! -e "$out" ]] || { echo 'Output already exists' >&2; exit 1; }
mkdir -p "$out"
out="$(cd "$out" && pwd)"
sdk="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}"
build="$sdk/build-tools/35.0.1"
java="${ANDROID_HUB_JAVA_HOME:-${P0_JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}}"
cat > "$out/AndroidManifest.xml" <<'XML'
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="local.androidhub.installsmoke" android:versionCode="1" android:versionName="1.0">
  <uses-sdk android:minSdkVersion="23" android:targetSdkVersion="35"/>
  <application android:label="Hub install smoke" android:hasCode="false"/>
</manifest>
XML
"$build/aapt2" link -o "$out/unsigned.apk" --manifest "$out/AndroidManifest.xml" -I "$sdk/platforms/android-35/android.jar"
"$build/zipalign" 4 "$out/unsigned.apk" "$out/aligned.apk"
"$java/bin/keytool" -genkeypair -keystore "$out/test.jks" -storepass testonly -keypass testonly -alias smoke -keyalg RSA -validity 2 -dname 'CN=Local Hub Test'
JAVA_HOME="$java" "$build/apksigner" sign --ks "$out/test.jks" --ks-pass pass:testonly --key-pass pass:testonly --out "$out/test.apk" "$out/aligned.apk"
echo "$out/test.apk"
