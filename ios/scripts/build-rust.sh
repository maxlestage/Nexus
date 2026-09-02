#!/usr/bin/env bash
# Compile controller-core en bibliothèque statique pour la plateforme visée
# par Xcode, puis la dépose là où l'éditeur de liens la cherche.
#
# Appelé automatiquement avant chaque build (preBuildScript du projet).
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="$PWD/build/rust"
mkdir -p "$OUT"

# PLATFORM_NAME est fourni par Xcode ; par défaut on vise l'iPhone.
case "${PLATFORM_NAME:-iphoneos}" in
  iphonesimulator)
    # Le simulateur tourne en arm64 sur Mac Apple Silicon, en x86_64 sinon.
    if [ "${NATIVE_ARCH:-arm64}" = "x86_64" ]; then
      TARGET="x86_64-apple-ios"
    else
      TARGET="aarch64-apple-ios-sim"
    fi
    ;;
  *) TARGET="aarch64-apple-ios" ;;
esac

PROFILE="release"
[ "${CONFIGURATION:-Release}" = "Debug" ] && PROFILE="debug"

echo "Cœur Rust : $TARGET ($PROFILE)"
rustup target add "$TARGET" >/dev/null 2>&1 || true

FLAGS=(-p nexus-app-core --target "$TARGET")
[ "$PROFILE" = "release" ] && FLAGS+=(--release)

# Xcode impose un SDKROOT qui perturbe la compilation des build-scripts Rust
# destinés à la machine hôte : on le neutralise pour cargo.
env -u SDKROOT -u SDK_DIR cargo build "${FLAGS[@]}" --manifest-path ../Cargo.toml

cp "../target/$TARGET/$PROFILE/libnexus_app_core.a" "$OUT/libnexus_app_core.a"
echo "  → $OUT/libnexus_app_core.a"
