# Mods: the plugin API

Mods are optional features compiled into Yappuccino. Each is off until the
person turns it on in **Settings → Mods**, and does only what its manifest's
permissions allow. Nothing is loaded at run time: no dynamic libraries, no
scripts, no WebAssembly, and no code from the internet. A runtime for
third-party code would cost more memory than Yappuccino's whole idle footprint
and would open a door this client keeps shut; see the decisions in
`AUDIT.md`. The catalog, and where each mod's behaviour comes from, is in
[mods.md](mods.md).

## Where it lives

| | |
| --- | --- |
| `crates/yappuccino-core/src/mods.rs` | `Manifest`, `Permission`, `Version`, the `CATALOG`, and `Mods`, the state `layout.json` keeps |
| `src/ui/mods.rs` | Settings → Mods, and the window callbacks each mod adds |
| `ui/mods.slint` | `ModsSettings` and `ModRow` |

## Manifest

```rust
Manifest {
    id: "message-utilities",          // stable, lowercase with dashes
    name: "Message Utilities",
    summary: "One sentence for Settings → Mods.",
    version: Version::new(1, 0, 0),
    api_version: API_VERSION,         // the host API it's written against
    min_app_version: Version::new(0, 3, 0),
    permissions: &[Permission::MessageMenu, Permission::Clipboard],
    inspiration: Some(Inspiration { name: "…", licence: "…" }),
}
```

`Manifest::compatibility(app)` says whether the mod runs in this Yappuccino:
`Compatible`, `NeedsApp(version)` when the app is older than
`min_app_version`, or `Api(n)` when the mod names another host API than
`API_VERSION` (1). A test checks that every manifest in `CATALOG` has a
unique id, at least one permission, and runs in the version being built.

## Permissions

A mod acts only where the host asks `Mods::allows(id, permission)`, which is
true while the mod is on and its manifest lists the permission. Settings →
Mods lists each mod's permissions under it, in words.

| Permission | Lets the mod |
| --- | --- |
| `MessageMenu` | Add items to a message's menu |
| `Clipboard` | Put text on the clipboard when the person picks one of those items |

New permissions come with the first mod that needs them, each with its
check at the one place the host does that thing.

## State

`layout.json` keeps the mods under `mods`:

```json
"mods": {
  "enabled": ["message-utilities"],
  "settings": { "message-utilities": { "format": "markdown" } }
}
```

- An id this Yappuccino doesn't ship, such as one from a newer version, is
  kept but never runs.
- A mod's settings stay when it's turned off, so turning it on finds them.
- A mod that fails while running is turned off for the rest of the run with
  `Mods::fail`, which logs why; it stays on in `layout.json` and runs again
  at the next start, or when turned off and on.
- Turning on a mod that isn't compatible fails with `ModError` and changes
  nothing.

## Adding a mod

1. Add its `Manifest` to `CATALOG` in `mods.rs`, with the fewest
   permissions that do the job, and a constant for its id.
2. Where the host does what the mod changes, ask
   `layout.mods.allows(ID, Permission::…)` first. In Slint, give the window
   a property that `mods::show` sets from `allows`, so the UI shows the
   mod's parts only while it's on.
3. Test it off, on, and remembered (`src/ui/tests.rs`,
   `mods_are_off_until_turned_on_and_act_only_while_on`).
4. Add its row to [mods.md](mods.md), with its inspiration and licence,
   and how it keeps to Discord's Terms.

A mod never sends a message, reacts, changes the account or its presence,
or fetches Discord's data on a timer or by itself: those happen only from
the person's own click, as anywhere else in Yappuccino.
