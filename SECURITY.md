# Security

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Use GitHub's private
security advisory form:

<https://github.com/juanmaramos/kbd.ctrl/security/advisories/new>

Include the affected version, macOS version, connection type, and enough detail
to reproduce the issue. Do not attach private keys, device backups, or logs that
contain personal information.

## Scope

kbd.ctrl has Input Monitoring and Accessibility access so it can recognize the
supported controller and translate its keys for the active app. It also writes
persistent configuration to one exact vendor HID interface.

Changes involving global input, synthesized events, file writes, or HID reports
receive security-sensitive review. The app does not require a cloud service and
does not include analytics.
