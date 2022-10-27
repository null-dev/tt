#!/bin/zsh
# ZSH is required because we use =()

alias adb='/home/nulldev/Desktop/Scripts/adb'

# Based on: package/pkg-cargo.mk and package/rustc/rustc.mk
BUILDROOT_OUTPUT="$(realpath ../cross/buildroot/output)"
TARGET_DIR="$BUILDROOT_OUTPUT/target"
HOST_DIR="$BUILDROOT_OUTPUT/host"
HOST_BIN_DIR="$HOST_DIR/bin"
BR_GCC_BASE="$HOST_BIN_DIR/arm-buildroot-linux-gnueabihf"

export CARGO_HOME="$HOST_DIR/share/cargo"
export CARGO_BUILD_TARGET="armv7-unknown-linux-gnueabihf"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="$BR_GCC_BASE-gcc"
export RUSTFLAGS="-Clink-arg=-Wl,--allow-multiple-definition"
export CARGO_UNSTABLE_TARGET_APPLIES_TO_HOST="true"
export CARGO_TARGET_APPLIES_TO_HOST="false"
# Make native build scripts use host CC
export CC_x86_64_unknown_linux_gnu="cc"

export AR="$BR_GCC_BASE-gcc-ar"
export AS="$BR_GCC_BASE-as"
export LD="$BR_GCC_BASE-ld"
export NM="$BR_GCC_BASE-gcc-nm"
export CC="$BR_GCC_BASE-gcc"
export GCC="$CC"
export CXX="$BR_GCC_BASE-g++"
export CPP="$BR_GCC_BASE-cpp"
export FC="$BR_GCC_BASE-gfortran"
export F77="$FC"
export RANLIB="$BR_GCC_BASE-gcc-ranlib"
export READELF="$BR_GCC_BASE-readelf"
export OBJCOPY="$BR_GCC_BASE-objcopy"
export OBJDUMP="$BR_GCC_BASE-objdump"
export PKG_CONFIG="$HOST_BIN_DIR/pkg-config"

cargo build --target "$CARGO_BUILD_TARGET" --release || exit 1

# Kill superbird
adb shell supervisorctl stop superbird

DEPLOY_ROOT="/tmp/tt"

adb shell rm -rf "$DEPLOY_ROOT" || exit 1
adb shell mkdir -p "$DEPLOY_ROOT" || exit 1
adb push target/armv7-unknown-linux-gnueabihf/release/tt "$DEPLOY_ROOT/" || exit 1
adb shell chmod +x "$DEPLOY_ROOT/tt"

# libgbm.so
adb push "$TARGET_DIR/usr/lib/libgbm.so.1.0.0" "$DEPLOY_ROOT/"
adb shell "cd '$DEPLOY_ROOT'; ln -s libgbm.so.1.0.0 libgbm.so.1"

# libinput.so
adb push "$TARGET_DIR/usr/lib/libinput.so.10.13.0" "$DEPLOY_ROOT/"
adb shell "cd '$DEPLOY_ROOT'; ln -s libinput.so.10.13.0 libinput.so.10"

# libevdev.so
adb push "$TARGET_DIR/usr/lib/libevdev.so.2.3.0" "$DEPLOY_ROOT/"
adb shell "cd '$DEPLOY_ROOT'; ln -s libevdev.so.2.3.0 libevdev.so.2"

read -r -d '' EXEC_SCRIPT << 'EOF'
cd "$(dirname "$0")"

# Global env vars
export LD_LIBRARY_PATH="$(realpath .)"

# Program-specific env vars
export WINIT_UNIX_BACKEND=fbdev

# Kill previous instance
PS_RESULT="$(ps)"
for pid in $(echo "$PS_RESULT" | grep '\./tt' | cut -d' ' -f 1); do
  echo "Killing old process: $pid"
  kill $pid
done

# Clear display
echo 1 > /sys/class/graphics/fb0/osd_clear

# Run
./tt "$@"
EOF

adb push =(echo "$EXEC_SCRIPT") "$DEPLOY_ROOT/exec.sh"
adb shell chmod +x "$DEPLOY_ROOT/exec.sh"

adb shell "$DEPLOY_ROOT/exec.sh"