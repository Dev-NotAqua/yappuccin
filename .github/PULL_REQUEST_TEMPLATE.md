## What and why

<!-- What changes for someone using Yappuccino, and why. Link the issue. -->

## Checks

- [ ] `scripts/check --fast` passes
- [ ] `scripts/test-fake-discord` passes, if this touches `crates/yappuccino-discord`, `discord_text` or `testing/fake_discord.ts`
- [ ] The doc for this area (`docs/`) says how it works now
- [ ] Memory: idle and the affected scenario measured as in `docs/measurements/README.md`, numbers in the commit, if this can change memory or CPU
- [ ] UI: works with the keyboard alone, keeps focus visible, and reads in a light and a dark theme
- [ ] Mods: the manifest lists every permission used; nothing sends messages or changes the account without a click

## Screenshots

<!-- Before and after, for UI changes. -->
