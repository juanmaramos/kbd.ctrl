# Codex Micro comparison and implementation scope

## What the official device does

The current Codex Micro integration is more than a collection of keyboard
shortcuts. The device is recognized by its own USB identifiers and talks to
Codex over a vendor HID protocol.

Its relevant controls include:

- Push-to-talk or Voice Chat.
- Accept and reject.
- New chat and send message.
- Increase and decrease reasoning effort.
- Conversation scrolling and composer navigation on the dial.
- Six agent keys that focus chats and show live states with color.
- Additional assignable Codex actions and skills.

The live agent colors, thread assignment, battery state, joystick skills, and
in-app configuration depend on the official hardware integration. Assigning
the same function keys to another keyboard does not make Codex recognize it as
a Codex Micro.

## What this pad can reproduce

The AliExpress pad has three layers. Each layer has six programmable physical
actions: three keys, knob left, knob right, and knob click. It therefore has up
to 18 stored actions, although only six are immediately accessible on the
active layer.

| Capability | Feasibility | Proposed implementation |
|---|---|---|
| Push-to-talk with press/release | High | Key 1 emits Control+F13 down/up; Codex exposes a native global hold-dictation command that requires a modifier |
| Approve | High | Key 2 emits F16; execute only while an approval prompt is detected |
| Stop current run | High | Key 3 emits F17; keep this available on the primary layer |
| Reasoning down/up | High | Knob left/right emit F18/F19 |
| Open model/effort picker | High | Knob click emits F20 |
| New chat, reject, send, review/terminal | High on another layer | Assign spare layer actions |
| Conversation scrolling | High | Use ordinary wheel output or map the dial contextually |
| Agent/chat selection | Partial | Use a layer for recent-chat shortcuts; no joystick is present |
| Live per-agent RGB status | Low for the first version | Requires both the vendor lighting protocol and reliable Codex task-state access |
| Native Codex hardware settings | Not available | This VID/PID is not recognized as official hardware |

The first five rows are the correct MVP. They preserve the original project
brief's useful six-key transport contract while avoiding unsupported claims about native
Codex integration.

## Recommended architecture

### Stage 1: prove the physical contract

1. Record the factory reports for every key and knob action.
2. Verify that holding a key produces distinct down and up events.
3. Read the current onboard configuration.
4. Write one reversible test mapping and confirm that it survives reconnecting.

Do not build the final user interface before these checks pass.

### Stage 2: configure Control+F13 and F16-F20

Build a narrow configurator for this exact `0x514c:0x8850` device. It should
read before writing, retain a backup of the original 64-byte reports, and
refuse unknown VID/PID or report layouts.

The vendor app writes 65-byte HID packets: report ID `3` followed by a 64-byte
configuration payload. The replacement configurator now backs up all three
layers, writes the six target records, commits, reads back, and verifies the
result.

### Stage 3: minimal macOS companion

Start with a small native menu-bar process rather than a full Tauri interface.
It needs:

- A global Control+F13 and F16-F20 event tap with down/up handling.
- Foreground-app detection.
- A Codex adapter with explicit command availability checks.
- A visible state indicator and an emergency disable control.
- A local event log containing actions and outcomes, never dictated text.

Use built-in shortcuts where Codex exposes them. For commands without a public
shortcut or API, a narrowly scoped Accessibility adapter is acceptable, but it
must verify the expected Codex UI state before acting.

### Stage 4: broaden only after the MVP works

Add other providers behind the same action vocabulary, then consider more
layers, dynamic RGB, and a polished configuration UI. Provider support should
be reported per action as supported, emulated, or unavailable rather than
pretending every tool has equivalent controls.

## Safety rules

- Never implement approval as a blind Enter key.
- Route approve through a provider's context-scoped approval command; never
  synthesize a blind Enter key.
- Stop must remain higher priority than all other actions.
- Disable automation in password and secure-input contexts.
- Do not capture ordinary keyboard text or microphone transcripts in logs.
- Do not write vendor HID reports until the current settings have been read and
  backed up.
