#!/bin/sh
# Hotfix build recipe: pure Rust; no CommonLib and no engine address patches.
set -eu
cd "$(dirname "$0")"
T="${SKSE_TOOLCHAIN_ROOT:-/mnt/h/dev/modding/toolchain}"
export PATH="$HOME/.cargo/bin:$T/bin:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$T/bin/lld-link"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static -L native=$T/xwin/crt/lib/x86_64 -L native=$T/xwin/sdk/lib/um/x86_64 -L native=$T/xwin/sdk/lib/ucrt/x86_64"
cargo +1.99.1 build --locked --release --lib --features ae --target x86_64-pc-windows-msvc -j "${BUILD_JOBS:-2}"
