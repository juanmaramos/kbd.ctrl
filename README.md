# kbd.ctrl

A macOS companion for the inexpensive three-key, one-dial macropad sold under
several generic names. kbd.ctrl safely configures the keypad once, then
translates its controls while Codex is active.

## What it does

| Control | Codex action |
| --- | --- |
| Hold the left key | Dictate into the composer |
| Middle key | Cancel, reject, or stop |
| Right key | Confirm, send, or approve |
| Turn the dial | Decrease or increase reasoning effort |
| Press the dial | Open the model picker |

The mapping is stored on all three hardware layers. Normal controls work over
USB or Bluetooth; configuration and RGB changes require USB.

Codex is the supported app in the first release. Claude Desktop support is
shown as coming soon until it passes the same end-to-end hardware tests.

## Supported hardware

The verified keypad identifies as:

- USB vendor ID `0x514c`
- USB product ID `0x8850`
- Product name `USB Composite Device`
- Three keys and one clickable rotary encoder

Products that look identical may use different firmware. kbd.ctrl only writes
to the exact supported HID interface and verifies every changed report.

The unit used for development came from this
[three-key, one-dial AliExpress listing](https://es.aliexpress.com/item/1005009473571709.html).
The link is provided only as a hardware reference; marketplace listings and
their internal firmware can change without notice.

## Install

Download the signed, notarized universal DMG from the
[latest GitHub release](https://github.com/juanmaramos/kbd.ctrl/releases/latest).

Homebrew installs the same notarized release artifact:

```sh
brew install --cask juanmaramos/tap/kbd-ctrl
```

## First setup

1. Connect the keypad over USB.
2. Open kbd.ctrl and follow the guided setup.
3. Grant Input Monitoring and Accessibility when macOS asks.
4. Let kbd.ctrl back up, configure, and verify the keypad.
5. In Codex Settings, assign the three shortcuts shown by kbd.ctrl.

The app can then live in the menu bar and start at login. macOS requires users
to grant both permissions themselves; kbd.ctrl cannot grant them silently.

## Privacy and safety

- kbd.ctrl runs locally and has no analytics or network service.
- Input events are used only to recognize the supported controller keys.
- Hardware writes are preceded by a backup and followed by read-back
  verification.
- The original backup is restored automatically if configuration fails.
- RGB is unavailable while this hardware revision operates over Bluetooth.

The vendor configuration package is not required. Its signatures did not
validate during inspection, so do not install it on a primary Mac. Technical
notes are in [docs/vendor-software-analysis.md](docs/vendor-software-analysis.md).

## Development

Requirements:

- macOS
- Node.js and npm
- Rust
- Xcode Command Line Tools
- An Apple Development certificate for permission-stable local testing

```sh
cd apps/desktop
npm ci
npm run app:dev:signed
```

The signed development app uses `com.rhams.kbdctrl.dev`. See
[docs/macos-permissions.md](docs/macos-permissions.md) before testing global
input.

Run the full local quality gate:

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

Hardware protocol and inspection notes live in
[docs/hardware-inspection.md](docs/hardware-inspection.md).
Maintainer signing, notarization, and Homebrew instructions are in
[docs/distribution.md](docs/distribution.md).

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before changing the hardware
protocol. Security issues should follow [SECURITY.md](SECURITY.md).

## License

kbd.ctrl is available under the [MIT License](LICENSE).
