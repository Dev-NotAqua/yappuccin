# Changelog

Notable changes to Yappuccino, newest first. Nightly and release notes on the
[releases page](https://github.com/Dev-NotAqua/yappuccino/releases) list every
commit; this file keeps what a person using the app would notice.

## Unreleased

### Added

- A new logo: a cappuccino seen from above, its foam poured as a speech
  bubble, on a marigold tile. The UI draws the same cup in the theme's accent.
- `python scripts/prepare-windows-skia.py --build` sets up the whole native
  Windows build itself: NASM, LLVM and its linker, and the prepared Skia.
- Code of conduct, security policy, issue and pull request templates, and
  checks that run on every pull request.

### Changed

- The tray's unread dot sits on the logo's top right.
- Links to the repository, releases and issue tracker point at
  `Dev-NotAqua/yappuccino`.

### Fixed

- A native Windows build on an x64 PC no longer picks Visual Studio's ARM64
  `clang-cl`, which can't run there.
- Native Windows builds keep Cargo's cache from one build to the next instead
  of recompiling FFmpeg, x264, ring and aws-lc each time.

## 0.4.0 nightlies and earlier

Yappuccino was called yapfast until 9 November 2026. Earlier history is in the
[releases](https://github.com/Dev-NotAqua/yappuccino/releases) and the Git log.
