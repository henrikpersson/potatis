#!/bin/sh
set -e

f=""
if [ $1 == "release" ] 
then
  f="--release"
fi

a=`echo ${1:0:1} | tr  '[a-z]' '[A-Z]'`${1:1}

cargo build $f --target aarch64-linux-android
cp ../target/aarch64-linux-android/$1/libnes_android.so android-app/app/src/main/jniLibs/arm64-v8a
cd android-app && ./gradlew install$a
adb shell am start -n nes.potatis/.MainActivity
