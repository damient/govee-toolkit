# Changelog

Changes to the desktop app in `apps/desktop`. It releases under
`desktop-vX.Y.Z`. The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- Studio in a window on macOS and Windows. The app runs the Studio server
  in its own process on the loopback port 4090.
- The release carries a `.dmg` for macOS on Apple silicon, and a
  `-setup.exe` for Windows. The release `desktop-latest` carries the same
  files under names without a version.
- Studio speaks English, French, German, Spanish and Italian. Until the
  user picks a language in Settings, the app takes the language of the
  system.
