#!/bin/bash

set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "Usage: $0 <version> <dmg-sha256> <output-path>" >&2
  exit 64
fi

version="$1"
sha256="$2"
output_path="$3"
script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_directory="$(cd "${script_directory}/.." && pwd)"
template_path="${project_directory}/packaging/homebrew/Casks/kbd-ctrl.rb.template"

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid semantic version: ${version}" >&2
  exit 65
fi

if [[ ! "${sha256}" =~ ^[0-9a-fA-F]{64}$ ]]; then
  echo "Invalid SHA-256: ${sha256}" >&2
  exit 65
fi

normalized_sha256="$(printf '%s' "${sha256}" | tr '[:upper:]' '[:lower:]')"

mkdir -p "$(dirname "${output_path}")"
sed \
  -e "s/__VERSION__/${version}/g" \
  -e "s/__SHA256__/${normalized_sha256}/g" \
  "${template_path}" > "${output_path}"

if grep -Eq '__VERSION__|__SHA256__' "${output_path}"; then
  echo "The rendered cask still contains placeholders." >&2
  exit 66
fi

echo "Rendered ${output_path}"
