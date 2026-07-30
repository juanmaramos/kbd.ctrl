# Distribution

kbd.ctrl ships as one universal macOS DMG from GitHub Releases. The same
versioned artifact is referenced by the Homebrew cask.

## Release credentials

The GitHub `release` environment requires:

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12` |
| `KEYCHAIN_PASSWORD` | Ephemeral CI keychain password |
| `APPLE_API_ISSUER` | App Store Connect issuer ID |
| `APPLE_API_KEY` | App Store Connect key ID |
| `APPLE_API_KEY_BASE64` | Base64-encoded App Store Connect `.p8` |

Authenticate GitHub CLI before adding them:

```sh
gh auth login -h github.com
gh auth status
```

Add private file contents without printing them:

```sh
base64 < /path/to/DeveloperIDApplication.p12 |
  gh secret set APPLE_CERTIFICATE \
    --env release \
    --repo juanmaramos/kbd.ctrl

base64 < /path/to/AuthKey_KEYID.p8 |
  gh secret set APPLE_API_KEY_BASE64 \
    --env release \
    --repo juanmaramos/kbd.ctrl
```

Enter passwords interactively so they do not enter shell history:

```sh
gh secret set APPLE_CERTIFICATE_PASSWORD \
  --env release \
  --repo juanmaramos/kbd.ctrl
openssl rand -base64 32 |
  gh secret set KEYCHAIN_PASSWORD \
    --env release \
    --repo juanmaramos/kbd.ctrl
```

The issuer and key identifiers can be entered interactively in the same way:

```sh
gh secret set APPLE_API_ISSUER \
  --env release \
  --repo juanmaramos/kbd.ctrl
gh secret set APPLE_API_KEY \
  --env release \
  --repo juanmaramos/kbd.ctrl
```

Never commit `.p8`, `.p12`, `.cer`, or their base64 representations.

## Publish a release

Keep these versions equal:

- `apps/desktop/package.json`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/tauri.conf.json`

Push a matching tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow:

1. Runs the source quality gate.
2. Builds a universal Apple Silicon and Intel app.
3. Signs with Developer ID Application.
4. Notarizes and staples the app and DMG.
5. Verifies Gatekeeper acceptance.
6. Publishes the DMG, checksum, and rendered Homebrew cask.

## Homebrew tap

Create `juanmaramos/homebrew-tap` once:

```sh
gh repo create juanmaramos/homebrew-tap \
  --public \
  --description "Homebrew casks for Juanma Ramos projects"
```

After a successful release, copy its generated `kbd-ctrl.rb` asset to
`Casks/kbd-ctrl.rb` in that repository. Then users can install with:

```sh
brew install --cask juanmaramos/tap/kbd-ctrl
```

Automating this final cross-repository update requires a fine-grained token
with Contents write access only to `juanmaramos/homebrew-tap`. Keep that token
in the main repository as `HOMEBREW_TAP_TOKEN`.
