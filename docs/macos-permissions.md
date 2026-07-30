# macOS permissions and signing

kbd.ctrl needs Input Monitoring to observe its transport keys while Codex,
Claude, or another supported app is frontmost.

macOS stores privacy grants against an app's code-signing designated
requirement, not its display name alone. An ad hoc signature is tied to one
specific build, so rebuilding an ad hoc-signed app invalidates the previous
grant even when the bundle identifier is unchanged.

For durable local testing:

1. Install an Apple Development certificate, including its private key, in the
   login keychain.
2. Confirm `security find-identity -v -p codesigning` lists the identity.
3. From `apps/desktop`, run `npm run app:dev:signed`. The script selects the
   single Apple Development certificate fingerprint, builds
   `kbd.ctrl Dev.app`, verifies its signature and designated requirement,
   and launches that bundle.
4. If the Mac has multiple Apple Development certificates, select one using its
   SHA-1 fingerprint:

   ```sh
   KBD_APPLE_SIGNING_IDENTITY="0123456789ABCDEF0123456789ABCDEF01234567" \
     npm run app:dev:signed
   ```

   To keep the selection for this checkout, place only that fingerprint in
   `apps/desktop/.signing-identity.local`. The `*.local` rule keeps this
   machine-specific file out of version control.

5. Enable `kbd.ctrl Dev` once in System Settings > Privacy & Security >
   Input Monitoring and Accessibility, then relaunch it.

The development flavor intentionally uses
`com.rhams.kbdctrl.dev`. CI release builds retain
`com.rhams.kbdctrl` and use Developer ID Application signing. macOS
will therefore keep development and production privacy grants separate.

Notarization is required for public distribution outside the Mac App Store, but
it does not replace Input Monitoring consent. A stable code-signing identity is
what lets that consent survive app updates.
