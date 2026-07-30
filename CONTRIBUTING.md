# Contributing to kbd.ctrl

Thanks for helping make inexpensive hardware useful.

## Before you start

- Open an issue before adding support for a new hardware identifier or changing
  the persistent HID protocol.
- Never commit certificates, private keys, notarization credentials, device
  backups, or user logs.
- Do not use the vendor configurator as an implementation dependency.

## Development

```sh
cd apps/desktop
npm ci
npm run app:dev:signed
```

The signed development command preserves the macOS identity associated with
Input Monitoring and Accessibility. See
[docs/macos-permissions.md](docs/macos-permissions.md).

## Pull requests

Keep changes focused and run:

```sh
cd apps/desktop
npm run typecheck
npm run lint
npm run format:check
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Hardware changes must preserve the write transaction:

1. Read and save a backup.
2. Write only reports for the exact supported interface.
3. Commit the device configuration.
4. Read every changed report back.
5. Restore the backup on failure.

Describe any physical USB and Bluetooth checks in the pull request.
