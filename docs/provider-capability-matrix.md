# Provider capability matrix

Verified on 2026-07-29 against the locally installed Codex app. Claude Desktop
support is experimental and did not pass the subsequent live hardware test.

| Action | Codex desktop | Claude Desktop Chat | Claude Desktop Code |
|---|---|---|---|
| Hold to dictate | Native OS-global command: `globalDictationHold` | Experimental | Not supported |
| Confirm / send | `Return` in the active context | Experimental | Experimental |
| Decline / stop | `Escape` in the active context | Experimental | Experimental |
| Decrease effort | `composer.decreaseReasoningEffort` | Experimental | Experimental |
| Increase effort | `composer.increaseReasoningEffort` | Experimental | Experimental |
| Choose model | `composer.openModelPicker` | Experimental | Experimental |

## Primary hardware profile

| Input | Provider-neutral intent |
|---|---|
| Control+F13 down/up | Hold to speak |
| F16 | Cancel, decline, or stop |
| F17 | Confirm, send, or approve |
| F18 | Move left or reduce effort |
| F19 | Move right or increase effort |
| F20 | Open the current app's model/effort picker |

## Codex setup contract

- Users assign `Control+F13` to Hold-to-dictate, `F18` to Decrease reasoning
  effort, and `F19` to Increase reasoning effort.
- `F16`, `F17`, and `F20` work through fixed companion translations to
  `Escape`, `Return`, and `Control+Shift+M`.
- Controller outputs are not user-configurable in the current release.
- Changing the three required assignments or Codex's default `Escape`,
  `Return`, or `Control+Shift+M` shortcuts can break the corresponding control.

The dial should operate on the choices the foreground application exposes. It
should not maintain a fake provider-independent model list.

## Routing policy

- Prefer a provider's context-scoped command binding.
- Translate Confirm and Cancel only while a known supported app is in front.
- Only translate inputs when the foreground app matches an enabled profile.
- Keep Control+F13 down and up distinct. Audio capture must never start
  before its key-down event or continue after its key-up event.
- Treat model and effort controls as surface-specific. If an app exposes only a
  picker, F20 opens it and the dial navigates that picker.
- Detect Claude Chat separately from Claude Code. Their shortcut namespaces
  overlap but do not have the same meaning.
- Unknown applications receive no confirm, reject, or model actions.

## Current implementation status

- Hardware Control+F13 and F16-F20 mapping: verified by configuration read-back.
- Foreground application detection: implemented in the desktop app.
- Codex command surface: verified in the installed app.
- Claude Desktop Chat bridge: experimental; it did not pass the live hardware
  test and is not advertised as supported.
- Claude Desktop Code command surface: implemented from Anthropic's documented
  shortcuts; live testing is pending an eligible Claude Code account.
- Global action execution: enabled when Input Monitoring and Accessibility
  access are granted to the running signed build.
