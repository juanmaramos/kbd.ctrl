# kbd.ctrl desktop

The Tauri v2 macOS app for [kbd.ctrl](../../README.md).

Use the permission-stable signed development build for hardware and global
input testing:

```sh
npm ci
npm run app:dev:signed
```

Raw `tauri dev` builds use an unstable signing identity and should not be used
to evaluate Input Monitoring or Accessibility permission persistence.

Before opening a pull request:

```sh
npm run typecheck
npm run lint
npm run format:check
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```
