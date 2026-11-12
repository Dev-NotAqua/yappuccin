# Yappuccino audit — 12 November 2026

A snapshot of the repository at `ead3335c` (branch `feat/new-logo`), taken
before the upgrade the plan below describes. Docs under `docs/` describe how
things work; this file records where things stood and what comes next, and
goes stale as the plan lands.

## Stack

| | |
| --- | --- |
| Language | Rust 2024, pinned 1.97.1 (`rust-toolchain.toml`) |
| UI | Slint at master `ac7d423f` (calls itself 1.19.0), Skia renderer, winit |
| Rendering | Skia Software by default, Skia GPU (Direct3D 12 on Windows, OpenGL on Linux) as a setting |
| Networks | Discord (default), Matrix (feature `matrix`) |
| Media | libwebrtc (LiveKit's `webrtc-sys`), vendored FFmpeg and x264, DeepFilterNet noise suppression on tract |
| Builds | Nix dev shell (Linux, and the Windows cross-build), native Windows for development (`scripts/prepare-windows-skia.py --build`) |
| Licence | GPL-3.0-or-later |
| Platforms | Linux, Windows 10/11. No macOS build (no test machine). |

Size, tracked Rust and Slint lines: `src/` 43k, `crates/yappuccino-core` 87k,
`crates/yappuccino-discord` 44k, `ui/` 26k, `crates/yappuccino-setup` 0.8k.

## Architecture as it is

`docs/architecture.md` is the source of truth; in short:

- `crates/yappuccino-core`: network-independent application state
  (`application/`), sessions, audio I/O, the Discord text pipeline
  (`discord_text/`), voice and streaming workers.
- `crates/yappuccino-discord`: Discord's REST, gateway, voice, cache, game
  detection (`game_detection/`: catalog, Steam, custom games), the fake-server
  test harness.
- `crates/yappuccino-ui`: the compiled Slint interface, kept **outside** the
  workspace on purpose: Clippy linting Slint's generated code took 190 s of a
  253 s check.
- `src/`: the desktop shell: windows, tray, notifications, push-to-talk,
  themes (`src/ui/theme.rs`), callbacks from `ui/*.slint` to `application/`.
- `crates/yappuccino-setup`: the Windows installer, in Rust and Slint.
- Settings live in `layout.json` (per machine) and Discord's own settings
  (synced). Secrets live in the OS credential store.

The workspace split already gives the boundaries the brief asks for (core,
network, UI, shell). Splitting it further into `settings`, `activities`,
`storage` crates would churn 200k lines and change compile times that were
tuned on purpose; see **Decisions**.

## What already covers the brief

| Brief item | Already there |
| --- | --- |
| Themes, custom colours | 10 built-in themes, a live theme editor, custom themes in `layout.json` (`docs/themes.md`) |
| Message text size, window zoom | Settings, Ctrl +/−/wheel (`docs/chat-ui.md`) |
| Quick switcher | Ctrl+K, recent conversations, fuzzy search (`docs/chat-ui.md`) |
| Copy message ID | Developer mode "Copy ID" items (off by default) |
| Search | Server-wide search with Discord's filters, pins |
| Notifications | Discord's per-server/category/channel levels and mutes, desktop toasts |
| Sounds | Six sound sets for join, leave, mute, message |
| Game detection | Linux/Windows/macOS process scan, Steam, Wine, Linux Rich Presence, custom game names, a sharing toggle |
| Privacy | QR sign-in, token in the credential store, no analytics, no update check |
| Docs | `docs/` per area, `docs/discord/` per Discord feature, measured resources |

## Gaps against the brief

- **Mods**: no plugin system or Mods tab. README promises plugins ("they're coming").
- **Activities**: one global sharing toggle; no per-game allow/block list,
  privacy mode or "only when I allow".
- **Personalization**: no density, font family, sidebar or message layout
  choices, configurable keybinds, or settings profile import/export.
- **Onboarding**: no first-run tour; the quick switcher has no app commands.
- **GitHub presence**: README, `CONTRIBUTING.md`, `LICENSE` exist; no
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`, issue/PR templates,
  or CI on pushes and pull requests (only a manual nightly).
- **Pages site**: none.

## Baseline

From `docs/measurements/README.md` (25 October 2026, 0.2.0 Linux release,
Skia Software, against `testing/fake_discord.ts`):

| | Memory | CPU |
| --- | --- | --- |
| Idle | 30 MB | 0.015% |
| Image-heavy channel | 41 MB | 0.008% |
| In a call | 44 MB | 0.33% |
| Sharing a screen | 113 MB | 1.1% |

These are the numbers every increment must not regress. They were measured on
Linux; this Windows machine has no measured baseline yet, and none is claimed.

## Hot spots and debt

- **Very large Slint files**: `ui/yappuccino.slint` 296 KB, `server.slint`
  132 KB, `messages.slint` 131 KB. Each `.slint` edit recompiles the UI crate:
  ~4 min and 6–8 GiB. New UI goes in new files under `ui/`.
- **Native Windows build was fragile**: Visual Studio's ARM64 `clang-cl` was
  picked on x64 hosts; the environment changed between builds and threw away
  Cargo's cache. Since fixed.
- **Rebrand leftovers**: README badges, release links, `CONTRIBUTING.md`,
  packages, the installer and invite examples pointed at yapfast's
  repository, website and Discord server. They now point at
  `cerbiss/yappuccin`, or are gone where Yappuccino has no such service.
- **CI**: nothing runs on pull requests, so the quality gates rest on
  `scripts/check` being run by hand.
- **Working tree**: uncommitted voice / Go Live work by someone else is in this
  checkout (`stream_worker.rs`, `voice.slint`, `desktop_capturer.cpp`, …).
  Increments here stage only their own files.

## Licence and Terms of Service

- **Upstream**: Yappuccino began as a fork of yapfast (GPL-3.0-or-later).
  Keeping the code means keeping the GPL, its notices, the source offer and
  the credit to yapfast's authors, which the README gives. "Not a copy of yapfast" applies to identity, UI and the site:
  new design work is ours; the inherited code stays credited.
- **Inspirations for mods**: Vencord is GPL-3.0, BetterDiscord is Apache-2.0.
  Nightcord's licence was not verified. No code or assets are taken from any
  of them; mods are written from their observed behaviour and cite it as a
  behavioural reference in `docs/mods.md`.
- **Discord's Terms**: third-party clients are against them, as the README
  already says. Mods must not add automation on top. Brief items changed for
  that reason:

| Brief item | Problem | Compliant version |
| --- | --- | --- |
| 8. Custom status **rotation** | Changes the account's status on a timer: automation | Saved presets applied with one click, with Discord's own "clear after" |
| 18. **Auto**-reply templates | Sends messages without the person | Reply templates that fill the composer; the person sends |
| 5. Hide **read receipts** | Discord has no read receipts; read state syncs unread markers | "Don't mark as read until I scroll to the bottom", local |
| 17. Local message backup | Paging a channel's history through the API is scraping | Export of messages already loaded in this client, opt-in |
| 19. Rich Presence composer | Arbitrary app IDs and buttons impersonate games | A "Playing …" name, as Discord's own "add game" does |
| 15. Translation | Sends message text to a third party | Per-message action, the person's own API key, off by default |

## Decisions

1. **Keep Slint and the current crates.** Add new areas as modules
   (`yappuccino-core/src/mods/`, `…/activities/`) and new `ui/*.slint` files,
   not new crates, unless a boundary needs its own compile unit.
2. **Mods are compiled in, not loaded.** Each mod is Rust in the binary, with a
   `ModManifest` (id, name, version, api version, permissions, minimum app
   version), off by default, enabled per person, settings in `layout.json`.
   Mods reach the app only through a host API that checks the manifest's
   permissions. Installable packages are data only: theme tokens, sound
   packs, status presets, reply templates. No dynamic libraries and no WASM
   in v1: either means running code the app didn't ship, and a WASM runtime
   alone would cost more memory than the 30 MB idle baseline allows. Revisit
   with measurements if third-party code mods are wanted.
3. **Docs follow `CLAUDE.md`**: one doc per area under `docs/`.
   `ARCHITECTURE.md` is `docs/architecture.md` (extended, not duplicated);
   new: `docs/settings.md`, `docs/plugins.md`, `docs/mods.md`,
   `docs/activities.md`, `docs/pages.md`. Root files are the GitHub
   community ones.
4. **CI on Linux and Windows only**, through the Nix shell the project
   already builds with (the Windows build is the cross-build). No macOS job
   until there is a macOS build.

## Plan

Each increment is one pull request, checked with `scripts/check --fast` (plus
`scripts/test-fake-discord` for Discord work) before it lands.

| # | Increment | Main files |
| --- | --- | --- |
| 0 | Native Windows build that keeps its cache; this audit | `scripts/prepare-windows-skia.py`, two `build.rs`, `docs/building.md` |
| 1 | GitHub presence: rebrand leftovers, Code of Conduct, Security policy, Changelog, issue/PR templates, CI on pull requests | `README.md`, `.github/` |
| 2 | Mods foundation: manifest, registry, permissions, persistence, Mods tab listing and toggling mods | `core/src/mods/`, `ui/mods.slint`, `docs/plugins.md`, `docs/mods.md` |
| 3 | Group A mods, one per increment, in the brief's order (adjusted as above) | `core/src/mods/*` |
| 4 | Activities: per-game allow/block list, privacy mode, Presence Filters | `yappuccino-discord/src/game_detection/`, `docs/activities.md` |
| 5 | Personalization: density, font family, layouts, keybinds, profile import/export | `ui/`, `src/ui/`, `docs/settings.md` |
| 6 | Onboarding and command palette commands | `ui/`, `docs/chat-ui.md` |
| 7 | Pages site, static, no trackers | `site/`, `.github/workflows/pages.yml`, `docs/pages.md` |
| 8 | Group B mods (v1.1), then Group C (v1.2) | `core/src/mods/*` |

Each increment that touches resources measures idle memory again and gives the
numbers in its commit, as `docs/measurements/README.md` describes.
