# Security policy

## Supported versions

Only the latest release and the latest nightly get fixes. Update before you
report, if you can.

## Reporting a vulnerability

Report it privately through
[GitHub's private vulnerability reporting](https://github.com/Dev-NotAqua/yappuccino/security/advisories/new).
Please don't open a public issue, post it on Discord, or include a working
exploit against other people's accounts.

Say what an attacker can do, on which version and platform, and the steps to
reproduce it. Never send a real Discord token, password or recovery code, yours
or anyone's: redact them from logs (`YAPPUCCINO_LOG` output can contain request
paths, but Yappuccino never logs tokens on purpose; if you find one in a log,
that's a bug worth reporting too).

We aim to answer within a week and to ship a fix, or a plan, within thirty
days. We'll credit you in the release notes unless you'd rather not be.

## What's in scope

- How Yappuccino stores and sends the login token (the system credential
  store, Discord only).
- `yappuccino://` and `discord://` links, the IPC server games talk to, and
  links opened from messages.
- Parsing of anything a server or another user sends: messages, embeds,
  pictures, video, voice and stream packets.
- The Windows installer and uninstaller.
- Mods and the permissions they declare, once mods ship.

## Out of scope

- Discord's own services and Terms. Third-party clients, this one included,
  are against Discord's Terms of Service; see the README's
  "Will Discord ban me?".
- Attacks that need someone who already runs code as you on your computer.
