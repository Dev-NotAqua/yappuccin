# Changelog

Notable changes to Yappuccino, newest first. Nightly and release notes on the
[releases page](https://github.com/cerbiss/yappuccin/releases) list every
commit; this file keeps what a person using the app would notice.

## Unreleased

### Added

- **Mods**: Settings → Mods lists optional features built into Yappuccino,
  each off until turned on, with the permissions it uses. The first is
  Message Utilities: Copy Message Link and Copy Message ID in a message's
  menu.
- **Notification Tweaks** mod: muted words and quiet hours for message
  notifications, with mentions coming through unless turned off.
- **Privacy Shield** mod: every picture and video starts covered, as a
  spoiler, until clicked.
- **Compact Mode** mod: less space between one author's messages and the
  next.
- A new logo: a cappuccino seen from above, its foam poured as a speech
  bubble, on a marigold tile. The UI draws the same cup in the theme's accent.
- `python scripts/prepare-windows-skia.py --build` sets up the whole native
  Windows build itself: NASM, LLVM and its linker, and the prepared Skia.
- Code of conduct, security policy, issue and pull request templates, and
  checks that run on every pull request.

### Changed

- The tray's unread dot sits on the logo's top right.
- Links to the repository, releases and issue tracker point at
  `cerbiss/yappuccin`.

### Fixed

- A native Windows build on an x64 PC no longer picks Visual Studio's ARM64
  `clang-cl`, which can't run there.
- Native Windows builds keep Cargo's cache from one build to the next instead
  of recompiling FFmpeg, x264, ring and aws-lc each time.

## 0.4.0 nightlies and earlier

Yappuccino began as a fork of [yapfast](https://github.com/yapfast/yapfast)
(GPL-3.0-or-later) and took its own name on 9 November 2026.
