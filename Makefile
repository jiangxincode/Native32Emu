# Makefile wrapper for the Native32Emu libretro core.
#
# The libretro buildbot CI templates invoke `make` and expect the resulting
# core to be named `native32emu_libretro.<ext>` in the directory pointed to by
# MAKEFILE_PATH (the repo root). The buildbot cross-compiles Windows and macOS
# targets from Linux/macOS hosts, so it exports a `platform` variable that we
# map to the matching Rust target triple. Cargo then builds for that target and
# we copy the artifact to the name the buildbot expects.
#
# Requires a Rust toolchain (cargo + rustup) on the build machine.

CORENAME := native32emu
TARGET   := $(CORENAME)_libretro
CARGO    ?= cargo
PROFILE  ?= release

ifeq ($(PROFILE),release)
	CARGO_PROFILE_FLAG := --release
else
	CARGO_PROFILE_FLAG :=
endif

# `platform` is set by the libretro CI templates: win64, win32, unix, osx,
# android. `ARCH`/`arch` distinguish 32/64-bit and architecture variants.
platform ?=
ARCH     ?=
arch     ?=

# Android requires a different build path: the NDK toolchain links against
# Bionic, so we drive cargo through `cargo-ndk`. RetroArch on Android also
# expects the core to be named `<corename>_libretro_android.so`.
IS_ANDROID :=

# Android API level to target (minimum supported). 21 covers Android 5.0+.
ANDROID_API ?= 21

# Map the libretro platform/arch to a Rust target triple and library naming.
ifeq ($(platform),android)
	IS_ANDROID := 1
	CORE_LIB   := $(TARGET)_android.so
	CARGO_LIB  := lib$(TARGET).so
	# `ARCH` selects the Android ABI; default to arm64-v8a.
	ifeq ($(ARCH),arm)
		RUST_TARGET := armv7-linux-androideabi
		NDK_ABI     := armeabi-v7a
	else ifeq ($(ARCH),arm64)
		RUST_TARGET := aarch64-linux-android
		NDK_ABI     := arm64-v8a
	else ifeq ($(ARCH),x86)
		RUST_TARGET := i686-linux-android
		NDK_ABI     := x86
	else ifeq ($(ARCH),x86_64)
		RUST_TARGET := x86_64-linux-android
		NDK_ABI     := x86_64
	else
		RUST_TARGET := aarch64-linux-android
		NDK_ABI     := arm64-v8a
	endif
else ifeq ($(platform),win64)
	RUST_TARGET := x86_64-pc-windows-gnu
	CARGO_LIB   := $(TARGET).dll
	CORE_LIB    := $(TARGET).dll
else ifeq ($(platform),win32)
	RUST_TARGET := i686-pc-windows-gnu
	CARGO_LIB   := $(TARGET).dll
	CORE_LIB    := $(TARGET).dll
else ifeq ($(platform),osx)
	ifeq ($(arch),arm)
		RUST_TARGET := aarch64-apple-darwin
	else
		RUST_TARGET := x86_64-apple-darwin
	endif
	CARGO_LIB := lib$(TARGET).dylib
	CORE_LIB  := $(TARGET).dylib
else ifeq ($(platform),unix)
	ifeq ($(ARCH),x86)
		RUST_TARGET := i686-unknown-linux-gnu
	else
		RUST_TARGET :=
	endif
	CARGO_LIB := lib$(TARGET).so
	CORE_LIB  := $(TARGET).so
else
	# No platform hint: build natively for the host. Detect OS for naming.
	UNAME_S := $(shell uname -s 2>/dev/null)
	RUST_TARGET :=
	ifeq ($(OS),Windows_NT)
		CARGO_LIB := $(TARGET).dll
		CORE_LIB  := $(TARGET).dll
	else ifeq ($(UNAME_S),Darwin)
		CARGO_LIB := lib$(TARGET).dylib
		CORE_LIB  := $(TARGET).dylib
	else
		CARGO_LIB := lib$(TARGET).so
		CORE_LIB  := $(TARGET).so
	endif
endif

ifeq ($(RUST_TARGET),)
	CARGO_TARGET_FLAG :=
	CARGO_OUT_DIR     := target/$(PROFILE)
else
	CARGO_TARGET_FLAG := --target $(RUST_TARGET)
	CARGO_OUT_DIR     := target/$(RUST_TARGET)/$(PROFILE)
endif

CARGO_OUT := $(CARGO_OUT_DIR)/$(CARGO_LIB)

.PHONY: all clean

all: $(CORE_LIB)

$(CORE_LIB):
ifneq ($(RUST_TARGET),)
	rustup target add $(RUST_TARGET) || true
endif
ifeq ($(IS_ANDROID),1)
	# Android links against the NDK toolchain; cargo-ndk wires up the linker
	# and sysroot for the selected ABI. Requires ANDROID_NDK_HOME to be set.
	$(CARGO) ndk -t $(NDK_ABI) -p $(ANDROID_API) build -p native32emu-libretro $(CARGO_PROFILE_FLAG)
else
	$(CARGO) build -p native32emu-libretro $(CARGO_PROFILE_FLAG) $(CARGO_TARGET_FLAG)
endif
	cp -f "$(CARGO_OUT)" "$(CORE_LIB)"

clean:
	$(CARGO) clean
	rm -f "$(CORE_LIB)"
