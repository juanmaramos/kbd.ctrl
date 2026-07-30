# Hardware inspection

## Confirmed on 2026-07-29

The connected unit is a full-speed USB 2.0 composite HID device:

| Property | Value |
|---|---|
| Vendor ID | `0x514c` (`20812`) |
| Product ID | `0x8850` (`34896`) |
| Product | `USB Composite Device` |
| Serial | Device-specific; intentionally omitted |
| USB version | `1.10` |
| Device version | `1.00` |
| Speed | 12 Mbit/s |

It has two USB HID interfaces:

1. Interface 0 is vendor-defined (`Usage Page 0xff00`, Usage `1`). It uses
   report ID `3` with 64-byte input and output payloads. This is almost
   certainly the persistent configuration channel.
2. Interface 1 is a boot keyboard plus mouse and consumer-control device. It
   supports keyboard report IDs `1` and `4`, mouse report ID `2`, and consumer
   control report ID `5`.

The standard interface can emit:

- Six-key and nine-key keyboard reports, including HID usages through `0xff`.
- Three mouse buttons, X/Y motion, and a wheel.
- Consumer-control usages through `0x02ff`.

This matches the manual's basic-key, combination, mouse/wheel, and multimedia
configuration options.

## Inspect the device

Run:

```sh
./scripts/inspect-hardware.sh
```

## Build and run the event listener

Build:

```sh
./scripts/build-diagnostic-listener.sh
```

First discover the factory mapping:

```sh
./.build/kbd-listener --discover | tee /tmp/kbd-discovery.jsonl
```

Press and release each of the three keys separately. Then rotate the knob one
detent left, one detent right, and click it once. Stop with Control-C.

Discovery mode logs numeric HID usages from this VID/PID only. It does not
listen to the built-in or normal keyboard and does not translate usages into
typed text.

After the pad is configured to emit Control+F13 and F16-F20, use restricted mode:

```sh
./.build/kbd-listener | tee /tmp/kbd-events.jsonl
```

Restricted mode records only Control+F13 and F16-F20, wheel motion, pointer buttons, and
consumer controls. Each record includes down/up state, modifiers, and the
foreground application's bundle identifier.

If macOS blocks access, grant Input Monitoring under System Settings > Privacy
& Security to the application launching the listener. In this Codex desktop
session that is likely ChatGPT/Codex; from a shell it is the terminal
application. The listener does not request Accessibility access and does not
inject events.

## Remaining hardware checks

- Verify that a held physical key produces one down report followed by one up
  report.
- Determine the factory mapping of all six physical actions.
- Confirm whether the user's unit has Bluetooth or 2.4 GHz hardware.
- Compare identifiers in wireless mode, if present.
- Confirm that saved mappings survive unplugging.
- Do not enter a bootloader or flash firmware without a recovery procedure.
