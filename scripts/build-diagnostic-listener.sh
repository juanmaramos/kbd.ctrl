#!/bin/sh

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output_dir="$project_root/.build"

mkdir -p "$output_dir"
mkdir -p "$output_dir/module-cache"

xcrun swiftc \
  -swift-version 5 \
  -module-cache-path "$output_dir/module-cache" \
  -framework AppKit \
  -framework IOKit \
  "$project_root/apps/diagnostic-listener/main.swift" \
  -o "$output_dir/kbd-listener"

printf '%s\n' "$output_dir/kbd-listener"
