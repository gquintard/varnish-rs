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
    cargo clippy --all-targets --workspace $({{just_executable()}} get-package-exclude-args) -- -D warnings

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
    cargo check --all-targets --workspace $({{just_executable()}} get-package-exclude-args)

# Default build
build:
    cargo build --all-targets --workspace $({{just_executable()}} get-package-exclude-args)

# build all
build-all-features:
    cargo build --all-targets --workspace $({{just_executable()}} get-package-exclude-args) --features "ffi,vsc"

# Run all tests
test *ARGS: build
    cargo test --all-targets --workspace $({{just_executable()}} get-package-exclude-args) {{ARGS}}

# Find the minimum supported Rust version. Install it with `cargo install cargo-msrv`
msrv:
    cargo msrv find --min 1.77 --component rustfmt -- {{just_executable()}} ci-test-msrv

# Find unused dependencies. Install it with `cargo install cargo-udeps`
udeps:
    cargo +nightly udeps --all-targets --workspace

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
    cargo build --all-targets --workspace
    cargo test --all-targets --workspace
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

# Run tests only relevant to the latest Varnish version
ci-test-extras: test-doc

# Run all tests as expected by CI
ci-test: rust-info test-fmt clippy test build-all-features

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

# Get the `--exclude <SPEC>` parameter for the cargo build/test/... command, depending on the installed version of Varnish
[private]
get-package-exclude-args:
    #!/usr/bin/env bash
    set -euo pipefail
    if {{just_executable()}} get-varnish-version 7.0 > /dev/null 2> /dev/null ; then
        echo ""
    else
        EXCLUDE="--exclude vmod_be --exclude vmod_vfp --exclude vmod_vdp --exclude vmod_test"
        echo "INFO: Due to older Varnish, running with: $EXCLUDE" >&2
        echo "$EXCLUDE"
    fi

# Get the version of Varnish installed on the system. If a version arg is provided, check that the installed version is at least that version.
get-varnish-version $required_version="":
    #!/usr/bin/env bash
    set -euo pipefail
    VARNISH_VER=$(dpkg-query -W -f='${source:Upstream-Version}\n' varnish-dev || echo "unknown")
    if [ -n "$required_version" ]; then
        if [ "$(printf "$required_version\n$VARNISH_VER" | sort -V | head -n1)" != "$required_version" ]; then
            echo "ERROR: Varnish version $required_version is required, but $VARNISH_VER is installed."
            exit 1
        else
            echo "Found varnish-dev package v$VARNISH_VER >= $required_version"
        fi
    elif [ "$VARNISH_VER" = "unknown" ]; then
        echo "ERROR: varnish-dev package was not found"
        exit 1
    else
        echo "Found varnish-dev package v$VARNISH_VER"
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
docker-run-60 *ARGS: (docker-build-ver "60lts") (docker-run-ver "60lts" ARGS)
