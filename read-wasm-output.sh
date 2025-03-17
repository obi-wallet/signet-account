#!/bin/bash

if ! command -v rustfilt &> /dev/null
then
    echo "Installing rustfilt..."
    cargo install rustfilt
    exit
fi

if ! command -v twiggy &> /dev/null
then
    echo "Installing twiggy..."
    cargo install twiggy
    exit
fi

echo "Building example wasm..."

cargo clean

cargo build --target wasm32-unknown-unknown --release

WASM=$(find target -iname "gatekeeper_message.wasm")

echo "Creating human readable wasm text from ${WASM}..."

rm -f target/gatekeeper_message.unmangle.wat

wasm-opt ${WASM} --print | rustfilt -o target/gatekeeper_message.unmangle.wat

echo "Human readable wasm text: target/gatekeeper_message.unmangle.wat"

twiggy paths ${WASM} > target/gatekeeper_message-call-paths.txt

echo "Call Path Analysis: target/pg-call-paths.txt"