#!/bin/sh

set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
build_dir="$project_dir/.build"
module_cache="$build_dir/module-cache"

mkdir -p "$build_dir" "$module_cache"

swiftc \
  -module-cache-path "$module_cache" \
  -framework IOKit \
  -framework CoreFoundation \
  "$project_dir/apps/config-reader/main.swift" \
  -o "$build_dir/kbd-config-reader"

printf '%s\n' "$build_dir/kbd-config-reader"
