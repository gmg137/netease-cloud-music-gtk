#!/bin/sh

export MESON_BUILD_ROOT="$1"
export MESON_SOURCE_ROOT="$2"
export CARGO_TARGET_DIR="$MESON_BUILD_ROOT"/target
export CARGO_HOME=${CARGO_HOME:="$HOME/.cargo"}
export OUTPUT="$3"
export BUILDTYPE="$4"
export APP_BIN="$5"
export CARGO_FEATURES="$6"

FEATURE_ARG=""
if [ -n "$CARGO_FEATURES" ]; then
    FEATURE_ARG="--features $CARGO_FEATURES"
fi

if [ $BUILDTYPE = "release" ]
then
    echo "RELEASE MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml --release $FEATURE_ARG && \
        cp "$CARGO_TARGET_DIR"/release/"$APP_BIN" "$OUTPUT"
else
    echo "DEBUG MODE"
    cargo build --manifest-path \
        "$MESON_SOURCE_ROOT"/Cargo.toml $FEATURE_ARG && \
        cp "$CARGO_TARGET_DIR"/debug/"$APP_BIN" "$OUTPUT"
fi

