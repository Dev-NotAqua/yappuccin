# Mods catalog

The mods Yappuccino ships, and the ones planned, each off until turned on in
Settings → Mods. How they work: [plugins.md](plugins.md).

No mod copies code or assets from another project. Where a mod does what
another client's plugin does, it's written from that plugin's behaviour,
named below as a behavioural reference, with that project's licence for the
record.

## Shipped

### Message Utilities

Adds **Copy Message Link** and **Copy Message ID** to a sent message's menu,
with or without Developer Mode. The link is Discord's own
(`https://discord.com/channels/<server or @me>/<channel>/<message>`), or
matrix.to's for a Matrix event; a message still sending has neither, and the
items don't show on it.

## Compliance matrix

| Mod | Inspiration (public) | Its licence | Clean-room | Permissions | Discord's Terms | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Message Utilities | Discord's own message menu | proprietary | yes, behaviour only | MessageMenu, Clipboard | Copies what the client already shows; sends nothing | shipped, 1.0.0 |
| Theme Engine | BetterDiscord themes, Vencord's themes | Apache-2.0, GPL-3.0 | yes | to come | Local look only | planned, v1 |
| Compact Mode | Discord's compact message display | proprietary | yes | to come | Local look only | planned, v1 |
| Notification Tweaks | Vencord's notification plugins | GPL-3.0 | yes | to come | Local; changes what notifies on this device | planned, v1 |
| Privacy Shield | Vencord's privacy plugins | GPL-3.0 | yes | to come | Local blur; typing and read state as Discord's own settings allow ("hide read receipts" becomes "don't mark read until I scroll") | planned, v1 |
| Command Palette | Discord's quick switcher | proprietary | yes | to come | Navigation only | planned, v1 |
| Presence Filters | Discord's per-game activity toggle | proprietary | yes | to come | Shares less than Discord would, never invents an activity | planned, v1 |
| Custom Status Extras | BetterDiscord status plugins | Apache-2.0 | yes | to come | Saved presets applied by a click; **no rotation on a timer** | planned, v1 |
| Message Search+ | — | — | yes | to come | Searches messages already loaded on this device | planned, v1 |
| Notification Sounds | Vencord's sound plugins | GPL-3.0 | yes | to come | Local sounds | planned, v1 |
| Better Embeds, Media Viewer, Keyboard Navigation+, Pin Manager | Vencord plugins of similar names | GPL-3.0 | yes | to come | Local display and navigation | planned, v1.1 |
| Translation | Vencord's Translate | GPL-3.0 | yes | to come | Per message, the person's own API key, off by default; text goes to the provider they choose | planned, v1.1 |
| Accessibility (TTS) | — | — | yes | to come | Speech on this device | planned, v1.1 |
| Local Message Backup | — | — | yes | to come | Exports only messages already loaded here, opt-in; no paging through history | planned, v1.2 |
| Reply Templates | — | — | yes | to come | Fills the composer; **the person sends**, never automatic | planned, v1.2 |
| Rich Presence Composer | Discord's "add game" | proprietary | yes | to come | A "Playing …" name only; no made-up app ids or buttons | planned, v1.2 |
| Scheduled Reminders | — | — | yes | to come | Local reminders; never sends | planned, v1.2 |

Nightcord's licence hasn't been checked; nothing in the catalog is based on
it.
