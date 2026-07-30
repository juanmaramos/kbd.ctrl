# Vendor software static analysis

Analyzed on 2026-07-29. No vendor executable was run or installed.

Source:

```text
http://szxiaozi.com/tool.zip
```

The server delivered a 39,375,694-byte archive last modified on 2025-06-27.

## Hashes

| File | SHA-256 |
|---|---|
| `tool.zip` | `e8407a5bb14f962bc812176be67d17aa50ba5db4dd31c05e9829b6afab3b8626` |
| `MACEN.zip` | `3824e49f19730c82ce5a60f725043036bf6ececd587cddacb4235a00d5f27011` |
| `WINEN.rar` | `aef46dfd7ba30d30f03c403af99471a46a4a5ce3c694710805fd22332f9ab2be` |
| `MINI_KEYBOARD.pkg` | `ceb61731c121a613b1e18c0204cd663c98d5f4c80e4fd8666642b0692893f883` |

## macOS package

- The installer package signature is invalid.
- Gatekeeper cannot validate the package.
- The contained app's signature is also invalid or has been modified.
- The signature metadata claims team ID `4Z759TG5T3`, but the signing authority
  is unavailable because validation fails.
- The app is Intel-only (`x86_64`), not a native Apple Silicon binary.
- The app declares macOS 10.12 as its minimum version.
- The package has no installer scripts and installs only
  `/Applications/MINI_KEYBOARD.app`.
- No kernel extension, system extension, launch agent, privileged helper,
  bootloader, or firmware image is present.
- The main application links Qt 5.12.9, IOKit, and HIDAPI 0.12.
- Static strings contain HID read/write and configuration-read functions.
- The configuration writer calls `hid_write` with a 65-byte buffer whose first
  byte is report ID `3`, matching the vendor interface descriptor.
- The device-info request is a 65-byte HID write beginning
  `03 FB FB FB`.
- Reading the three-key/one-knob configuration uses a 65-byte request beginning
  `03 FA 19 00 <layer>`, where `<layer>` is `01`, `02`, or `03`. The device
  returns 25 64-byte records per layer.
- A configuration write is report ID `03` followed by one of those 64-byte
  records. A save is committed with `03 FD FE FF` followed by zeroes.
- The three keys are records `01`, `02`, and `03`. Encoder left, press, and
  right are records `10`, `11`, and `12` in hexadecimal.
- For an unmodified basic key, byte 9 of the 65-byte write packet contains the
  USB HID keyboard usage.
- No obvious HTTP URL, update service, or firmware-update function was found
  in the main executable's printable strings. Bundling QtNetwork alone does not
  prove network activity.

## Windows archive

The Windows archive is a Qt application containing `MINI_KEYBOARD.exe`,
`hidapi.dll`, Qt 5 DLLs, and image/platform plugins. It was listed but not
executed.

## Assessment

The payload is consistent with a simple HID configurator, and the installer
does not contain persistence or privileged scripts. However, the broken
signatures remove the normal macOS chain of trust. Do not install it on the
primary machine.

Preferred next steps:

The replacement configurator now reads and fingerprints all 76 reports, saves a
backup before each write, programs Control+F13 and F16-F20 across all three layers, commits,
then reads the device back and verifies every target record. If verification
fails, it restores the six target records from the pre-write backup.

The vendor application is no longer needed for this hardware revision.
