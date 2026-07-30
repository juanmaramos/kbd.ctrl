#!/bin/bash

set -euo pipefail

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_directory="$(cd "${script_directory}/.." && pwd)"
desktop_directory="${project_directory}/apps/desktop"
config_path="src-tauri/tauri.dev-signing.conf.json"
app_path="${desktop_directory}/src-tauri/target/debug/bundle/macos/kbd.ctrl Dev.app"
executable_path="${app_path}/Contents/MacOS/kbd-ctrl"
identity_file="${desktop_directory}/.signing-identity.local"

signing_identity="${KBD_APPLE_SIGNING_IDENTITY:-}"
if [[ -z "${signing_identity}" && -f "${identity_file}" ]]; then
  signing_identity="$(sed -n '1p' "${identity_file}")"
fi

if [[ -z "${signing_identity}" ]]; then
  signing_identities="$(
    security find-identity -v -p codesigning |
      sed -n 's/.*) \([0-9A-F]\{40\}\) "Apple Development:[^"]*".*/\1/p' |
      sort -u
  )"
  identity_count="$(printf '%s\n' "${signing_identities}" | sed '/^$/d' | wc -l | tr -d ' ')"

  if [[ "${identity_count}" == "0" ]]; then
    echo "No Apple Development signing identity was found." >&2
    echo "Create one in Xcode > Settings > Accounts > Manage Certificates." >&2
    exit 1
  fi

  if [[ "${identity_count}" != "1" ]]; then
    echo "More than one Apple Development certificate is available:" >&2
    printf '%s\n' "${signing_identities}" >&2
    echo "Set KBD_APPLE_SIGNING_IDENTITY to its SHA-1 fingerprint, or save the fingerprint in ${identity_file}." >&2
    exit 1
  fi

  signing_identity="${signing_identities}"
fi

identity_details="$(
  security find-identity -v -p codesigning |
    grep -F "${signing_identity}" |
    head -n 1
)"
signing_authority="$(printf '%s\n' "${identity_details}" | sed -n 's/.*"\([^"]*\)".*/\1/p')"
if [[ -z "${signing_authority}" ]]; then
  echo "The selected Apple Development identity is not currently valid: ${signing_identity}" >&2
  exit 1
fi

echo "Building with ${signing_authority} (${signing_identity})"
(
  cd "${desktop_directory}"
  APPLE_SIGNING_IDENTITY="${signing_identity}" \
    npm run tauri -- build --debug --bundles app --config "${config_path}"
)

codesign --verify --deep --strict --verbose=2 "${app_path}"
signature_details="$(codesign --display --verbose=4 "${app_path}" 2>&1)"
if ! printf '%s\n' "${signature_details}" | grep -Fq "Authority=${signing_authority}"; then
  echo "The app was not signed by the selected Apple Development identity." >&2
  printf '%s\n' "${signature_details}" >&2
  exit 1
fi

designated_requirement="$(codesign --display --requirements - "${app_path}" 2>&1)"
if ! printf '%s\n' "${designated_requirement}" |
  grep -Fq 'identifier "com.rhams.kbdctrl.dev"'; then
  echo "The signed app has an unexpected designated requirement." >&2
  printf '%s\n' "${designated_requirement}" >&2
  exit 1
fi

running_pid="$(pgrep -f -x "${executable_path}" || true)"
if [[ -n "${running_pid}" ]]; then
  kill "${running_pid}"
fi

open -n "${app_path}"

echo
echo "Signed app: ${app_path}"
printf '%s\n' "${designated_requirement}"
