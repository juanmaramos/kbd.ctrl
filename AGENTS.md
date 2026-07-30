# kbd.ctrl agent notes

## Purpose

kbd.ctrl is a Tauri v2 macOS companion for a generic three-key and
clickable-knob keypad. It keeps a universal hardware mapping and translates it
for the active supported AI app.

## Hardware contract

- USB device: `0x514c:0x8850`.
- The vendor HID interface (`0xff00:0x0001`) is for persistent configuration.
- USB is required for mappings, RGB changes, backup, and restore.
- Bluetooth or USB can provide normal daily keyboard input.
- Universal mapping on all three hardware layers:
  - Left key: `F13` — Speak
  - Middle key: `F16` — Cancel
  - Right key: `F17` — Confirm
  - Dial left/right: `F18` / `F19` — Effort
  - Dial click: `F20` — Models
- Always back up before a device write, commit, read back, verify, and restore
  the backup on failure.
- Do not attempt firmware flashing without identifying the MCU, obtaining a
  factory-flash backup, and establishing a tested recovery procedure.

## macOS development

Run the permission-stable development app from `apps/desktop`:

```sh
npm run app:dev:signed
```

This builds and launches `kbd.ctrl Dev.app` with identifier
`com.rhams.kbdctrl.dev`. It uses an Apple Development certificate
fingerprint from `.signing-identity.local` or
`KBD_APPLE_SIGNING_IDENTITY`. The local file is ignored; never commit
certificates, private keys, notarization keys, or passwords.

Input Monitoring and Accessibility grants belong to the signed development
identity. Ad-hoc rebuilds invalidate them. CI releases will instead use
`com.rhams.kbdctrl`, Developer ID Application signing, and notarization.

## RGB behavior

- Speak is purple, Cancel red, and Confirm green.
- The dial has no addressable RGB on this hardware revision.
- Reactive mode is a short firmware-controlled keypress pulse.
- Static mode keeps role colors visible.
- This hardware revision disables RGB lighting while operating over Bluetooth;
  lighting is available only while USB is connected.
- The companion cannot hold an LED for dictation or pending approval because
  the keypad exposes no live RGB channel over Bluetooth and Codex exposes no
  reliable pending-request state.

## Verification

Before handing off changes, run:

```sh
cd apps/desktop
npm run lint
npm run typecheck
npm run format:check
npm test
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test
```

For permission-sensitive testing, use the signed development command rather
than the raw `tauri dev` executable.
