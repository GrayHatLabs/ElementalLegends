#!/usr/bin/env bash
# Boots a headless Android 14 emulator (x86_64), installs dist/ElementalLegends.apk, starts it and
# saves screenshots to dist/android_test*.png (and the app's log to dist/android_log.txt).
# NOBUILD=1 skips rebuilding the APK. SCREEN=1280x960 (4:3, the default) or SCREEN=1920x1080.
set -e
cd "$(dirname "$0")/.."
export ANDROID_HOME=/opt/android-sdk
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
if ! avdmanager list avd | grep -q elemental_test; then
  echo no | avdmanager create avd -n elemental_test -k "system-images;android-34;google_apis;x86_64" -d pixel_6 > /dev/null
fi
SCREEN=${SCREEN:-1280x960}
SW=${SCREEN%x*}; SH=${SCREEN#*x}
CFG=$HOME/.android/avd/elemental_test.avd/config.ini
sed -i "/^hw.lcd.width=/d; /^hw.lcd.height=/d; /^hw.lcd.density=/d" "$CFG"
printf 'hw.lcd.width=%s\nhw.lcd.height=%s\nhw.lcd.density=240\n' "$SW" "$SH" >> "$CFG"
nohup emulator -avd elemental_test -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -no-snapshot > /tmp/emu_el.log 2>&1 &
adb wait-for-device
until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do sleep 3; done
[ -n "$NOBUILD" ] || { bash scripts/build-android.sh > /tmp/abuild_el.log 2>&1 || { tail -30 /tmp/abuild_el.log; exit 1; }; }
# A clean install each run: updating an installed copy can restart the app while it starts.
adb uninstall org.grayhatlabs.elementallegends > /dev/null 2>&1 || true
adb install dist/ElementalLegends.apk
sleep 8
adb logcat -c
# Skip Android's one-time "Viewing full screen" tip (a real device shows it once).
adb shell settings put secure immersive_mode_confirmations confirmed
adb shell am start -n org.grayhatlabs.elementallegends/.ElementalActivity
sleep 25
adb exec-out screencap -p > dist/android_test.png
# Enter (like a pad's Start) on the title menu, then a second screenshot.
adb shell input keyevent KEYCODE_ENTER
sleep 4
adb exec-out screencap -p > dist/android_test2.png
adb logcat -d -s SDL SDL/APP AndroidRuntime DEBUG | tail -60 > dist/android_log.txt || true
adb logcat -d | grep -iE "elementallegends|ActivityTaskManager|wm_" | tail -60 > dist/android_sys_log.txt || true
adb emu kill > /dev/null 2>&1 || true
echo "screenshots dist/android_test.png dist/android_test2.png"
