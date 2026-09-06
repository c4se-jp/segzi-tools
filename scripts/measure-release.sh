#!/bin/sh
set -eu

target_dir=${CARGO_TARGET_DIR:-target/measure-release}
CARGO_TARGET_DIR=$target_dir cargo build --release --locked

binary=$target_dir/release/segzify
bytes=$(wc -c < "$binary" | tr -d ' ')

printf 'artifact\tbytes\n'
printf '%s\t%s\n' "$binary" "$bytes"
