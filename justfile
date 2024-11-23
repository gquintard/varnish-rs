#!/usr/bin/env just --justfile

@_default:
    just --list --unsorted

# Clean all build artifacts
clean:
    cargo clean
    rm -f Cargo.lock

# Update dependencies, including breaking changes
update:
    cargo +nightly -Z unstable-options update --breaking
    cargo update

# Run cargo clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Test code formatting
test-fmt:
    cargo fmt --all -- --check

# Run cargo fmt
fmt:
    cargo +nightly fmt -- --config imports_granularity=Module,group_imports=StdExternalCrate

# Build and open code documentation
docs:
    cargo doc --no-deps --open

# Quick compile
check:
    cargo check --workspace --all-targets

# Default build
build:
    cargo build --workspace --all-targets

# build all
build-all-features:
    cargo build --workspace --all-targets --features "ffi,vsc"

# Run all tests
test *ARGS: build
    cargo test --workspace --all-targets {{ARGS}}

# Find the minimum supported Rust version. Install it with `cargo install cargo-msrv`
msrv:
    cargo msrv find --min 1.77 --component rustfmt -- {{just_executable()}} ci-test-msrv

# Find unused dependencies. Install it with `cargo install cargo-udeps`
udeps:
    cargo +nightly udeps --workspace --all-targets

# Check semver compatibility with prior published version. Install it with `cargo install cargo-semver-checks`
semver *ARGS:
    cargo semver-checks {{ARGS}}

# Generate and show coverage report. Requires grcov to be installed.
grcov:
    #!/usr/bin/env bash
    set -euo pipefail
    find . -name '*.profraw' | xargs rm
    rm -rf ./target/debug/coverage
    export LLVM_PROFILE_FILE="varnish-%p-%m.profraw"
    export RUSTFLAGS="-Cinstrument-coverage"
    cargo build --workspace --all-targets
    cargo test --workspace --all-targets
    grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
    open ./target/debug/coverage/index.html

# Publish crates to crates.io in the right order
publish *ARGS:
    cargo publish -p varnish-sys {{ARGS}}
    cargo publish -p varnish-macros {{ARGS}}
    cargo publish -p varnish {{ARGS}}

# Use the experimental workspace publishing with --dry-run. Requires nightly Rust.
test-publish:
    cargo +nightly -Z package-workspace publish --dry-run

# Run tests, and accept their results. Requires insta to be installed.
bless:
    TRYBUILD=overwrite cargo insta test -p varnish-macros -p varnish --accept --unreferenced=delete

# Test documentation
test-doc:
    cargo test --doc
    cargo doc --no-deps

rust-info:
    rustc --version
    cargo --version

# Run all tests as expected by CI
ci-test: rust-info test-fmt clippy test test-doc build-all-features

# Run minimal subset of tests to ensure compatibility with MSRV
ci-test-msrv: rust-info test

# Verify that the current version of the crate is not the same as the one published on crates.io
check-if-published:
    #!/usr/bin/env bash
    set -euo pipefail
    LOCAL_VERSION="$(grep '^version =' Cargo.toml | sed -E 's/version = "([^"]*)".*/\1/')"
    echo "Detected crate version:  $LOCAL_VERSION"
    CRATE_NAME="$(grep '^name =' Cargo.toml | head -1 | sed -E 's/name = "(.*)"/\1/')"
    echo "Detected crate name:     $CRATE_NAME"
    PUBLISHED_VERSION="$(cargo search ${CRATE_NAME} | grep "^${CRATE_NAME} =" | sed -E 's/.* = "(.*)".*/\1/')"
    echo "Published crate version: $PUBLISHED_VERSION"
    if [ "$LOCAL_VERSION" = "$PUBLISHED_VERSION" ]; then
        echo "ERROR: The current crate version has already been published."
        exit 1
    else
        echo "The current crate version has not yet been published."
    fi

[private]
docker-build-ver VERSION:
    docker build --progress=plain -t varnish-img-{{VERSION}} --build-arg VARNISH_VERSION={{VERSION}} --build-arg USER_UID=$(id -u) --build-arg USER_GID=$(id -g) -f docker/Dockerfile docker

[private]
docker-run-ver VERSION *ARGS:
    mkdir -p docker/.cache/{{VERSION}}
    touch docker/.cache/{{VERSION}}/.bash_history
    docker run --rm -it \
        -v "$PWD:/app/" \
        -v "$PWD/docker/.cache/{{VERSION}}:/home/user/.cache" \
        -v "$PWD/docker/.cache/{{VERSION}}/.bash_history:/home/user/.bash_history" \
        varnish-img-{{VERSION}} {{ARGS}}

docker-run-76 *ARGS: (docker-build-ver "76") (docker-run-ver "76" ARGS)
docker-run-75 *ARGS: (docker-build-ver "75") (docker-run-ver "75" ARGS)
docker-run-74 *ARGS: (docker-build-ver "74") (docker-run-ver "74" ARGS)
docker-run-60 *ARGS: (docker-build-ver "60") (docker-run-ver "60" ARGS)
