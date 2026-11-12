#!/usr/bin/env sh
# Builds the web version into dist/. Used by CI and for local testing.
# Needs on PATH: wasm-bindgen (same version as the wasm-bindgen crate in
# Cargo.lock) and wasm-opt (binaryen). Serve with:
#   python -m http.server -d dist   (or any static file server)
set -eu
cd "$(dirname "$0")/.."

cargo build --profile web --target wasm32-unknown-unknown -p oriflamme-rs

rm -rf dist
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section --out-dir dist --out-name oriflamme-rs \
    target/wasm32-unknown-unknown/web/oriflamme-rs.wasm

before=$(wc -c < dist/oriflamme-rs_bg.wasm)
wasm-opt -Oz --strip-debug --strip-producers \
    --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    -o dist/oriflamme-rs_bg.wasm dist/oriflamme-rs_bg.wasm
after=$(wc -c < dist/oriflamme-rs_bg.wasm)
gzipped=$(gzip -9 -c dist/oriflamme-rs_bg.wasm | wc -c)

cp web/index.html assets/icon.png dist/
echo "wasm: $before bytes before wasm-opt, $after after, $gzipped gzipped"
