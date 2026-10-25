# Main Menu Randomizer SE 2.0.4 Rust hotfix build

No CommonLib is used or needed. Prepare the workbench xwin SDK/CRT and LLVM 21 linker, then install the pinned Rust toolchain with rustup:

    rustup toolchain install 1.99.1 --profile minimal --target x86_64-pc-windows-msvc
    sh work/mainmenurandomizer-2.0.4/build-linux.sh

The build script uses Cargo.lock unchanged, --locked, --lib, Release, feature ae and x86_64-pc-windows-msvc with static CRT. Output: target/x86_64-pc-windows-msvc/release/main_menu_randomizer.dll.
Build only the library: the standalone replacer tool is not distributed in the TEST archive.

For isolated Linux host smoke tests only:

    cargo +1.99.1 build --locked --release --bin replacer --features ae --target x86_64-unknown-linux-gnu
    python3 tests/smoke-isolated.py target/x86_64-unknown-linux-gnu/release/replacer

Those tests use synthetic temporary files and do not load the Windows DLL or touch game files. The missing benchmark declaration was disabled because its source is absent from the published archive. No benchmark stub or replacement behaviour was invented.

Rust source and lockfile are unchanged. No explicit v5 flag was added to SKSE metadata. The DLL passes the workbench checker through its post-cutoff build timestamp; its filesystem-only implementation does not consume the Address Library despite the historical metadata bit. This does not verify the actual game load.

Record toolchain versions/SDK choices for repeats. Exact byte-identical rebuilds are not claimed (PE timestamp and environment may vary). See SOURCE-PROVENANCE.md before redistributing.
