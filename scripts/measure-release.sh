#!/bin/sh
set -eu

cargo build --release --locked

binary=target/release/segzify
bytes=$(wc -c < "$binary" | tr -d ' ')

printf 'artifact\tbytes\n'
printf '%s\t%s\n' "$binary" "$bytes"
