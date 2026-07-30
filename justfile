default:
    @just --list

# Format all code (rustfmt nightly + taplo)
fmt:
    cargo +nightly fmt --all

# Check formatting without changes
fmt-check:
    cargo +nightly fmt --all -- --check

# Build all targets
build:
    cargo build --all-features
    cargo build --all-features --target wasm32-unknown-unknown
    # RUSTFLAGS="-Ctarget-feature=+atomics" cargo +nightly build --all-features --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std  # cargo#7359

# Build docs
docs:
    cargo doc --no-deps --document-private-items --lib --examples --all-features
    RUSTDOCFLAGS="--cfg=web_sys_unstable_apis" RUSTFLAGS="--cfg=web_sys_unstable_apis" cargo doc --no-deps --document-private-items --lib --examples --all-features --target wasm32-unknown-unknown
    # -Zbuild-std variant omitted: cargo#7359 (duplicate lang item core)

# Run all native tests
test-native:
    cargo test --all-targets --no-fail-fast
    cargo test --doc --no-fail-fast

# Run wasm tests in Chrome (single-threaded, all features)
test-wasm-chrome DRIVER="chromedriver" FLAGS="--cfg=unsupported_spawn_then_block":
    CHROMEDRIVER={{DRIVER}} RUSTFLAGS="--cfg=web_sys_unstable_apis {{FLAGS}}" cargo test --all-features --target wasm32-unknown-unknown

# Run wasm tests in Firefox (single-threaded, all features)
test-wasm-firefox DRIVER="geckodriver":
    GECKODRIVER={{DRIVER}} RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" cargo test --all-features --target wasm32-unknown-unknown

# Run wasm doctests in Chrome (single-threaded)
test-wasm-doctest-chrome DRIVER="chromedriver" FLAGS="--cfg=unsupported_spawn_then_block":
    CHROMEDRIVER={{DRIVER}} RUSTFLAGS="--cfg=web_sys_unstable_apis {{FLAGS}}" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis {{FLAGS}}" cargo +nightly test --doc --all-features --target wasm32-unknown-unknown

# Run wasm tests with atomics in Chrome
test-wasm-atomics-chrome DRIVER="chromedriver":
    # Disabled: cargo#7359 (duplicate lang item core with -Zbuild-std + pre-installed target)
    # CHROMEDRIVER={{DRIVER}} RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn_then_block -Ctarget-feature=+atomics" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn_then_block -Ctarget-feature=+atomics" cargo +nightly test --all-features --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# Run wasm tests in Safari (macOS only)
test-wasm-safari DRIVER="safaridriver":
    SAFARIDRIVER={{DRIVER}} RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn_then_block --cfg=unsupported_shared_block" cargo test --all-features --target wasm32-unknown-unknown

# Run compile tests on wasm
test-compile-wasm:
    UI_TEST_TARGET=wasm32-unknown-unknown cargo test --test compile_test

# Run compile tests on wasm with atomics
test-compile-wasm-atomics:
    # Disabled: cargo#7359
    # UI_TEST_TARGET=wasm32-unknown-unknown UI_TEST_RUSTFLAGS="-Ctarget-feature=+atomics" UI_TEST_ARGS="--features message" UI_TEST_BUILD_STD=1 cargo +nightly test --test compile_test

# Run minimal versions check (MSRV)
test-minimal-versions:
    cd minimal-versions && cargo +nightly update -Zminimal-versions && cargo build --all-features --target wasm32-unknown-unknown

# Run all CI checks locally
ci: fmt-check build docs test-native
    @echo "All CI checks passed"

# Full CI including wasm browser tests (requires browsers installed)
ci-full: ci test-wasm-chrome test-wasm-firefox test-compile-wasm
    @echo "Full CI checks passed"
