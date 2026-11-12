//! Mods: optional features compiled into Yappuccino, each off until the person
//! turns it on in Settings → Mods. A mod is described by its [`Manifest`]
//! and reaches the app only where the host asks [`Mods::allows`] for one of
//! the [`Permission`]s the manifest lists, so a mod can't do what it didn't
//! declare. Nothing is loaded at run time: no dynamic libraries, no scripts.
//! See `docs/plugins.md` for the API and `docs/mods.md` for the catalog.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The version of the host API mods are written against. A manifest naming
/// another one doesn't run.
pub const API_VERSION: u32 = 1;

/// What a mod may do, each granted only while the mod is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    /// Add items to a message's menu.
    MessageMenu,
    /// Put text on the clipboard when the person picks one of those items.
    Clipboard,
    /// Keep a message's sound, desktop notification and taskbar flash on
    /// this device; never a call's ringing.
    Notifications,
    /// Cover pictures and videos in messages until they're clicked.
    MediaDisplay,
    /// Change how densely messages are laid out.
    MessageLayout,
}

impl Permission {
    /// What Settings → Mods says the permission lets the mod do.
    pub fn describe(self) -> &'static str {
        match self {
            Self::MessageMenu => "Add items to message menus",
            Self::Clipboard => "Copy text to the clipboard when you pick one",
            Self::Notifications => "Quiet message notifications on this device",
            Self::MediaDisplay => "Cover pictures and videos until you click them",
            Self::MessageLayout => "Lay out messages more densely",
        }
    }
}

/// A version as major.minor.patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Reads `1.2.3`, ignoring a pre-release or build suffix such as
    /// `-nightly.20261109.1`.
    pub fn parse(text: &str) -> Option<Self> {
        let core = text.split(['-', '+']).next()?;
        let mut parts = core.split('.').map(str::parse::<u32>);
        let version = Self::new(
            parts.next()?.ok()?,
            parts.next()?.ok()?,
            parts.next()?.ok()?,
        );
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The project whose behaviour a mod reimplements, written from what it does
/// rather than from its code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inspiration {
    pub name: &'static str,
    pub licence: &'static str,
}

/// What a mod is and what it may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Stable, lowercase with dashes; kept in `layout.json`.
    pub id: &'static str,
    pub name: &'static str,
    /// One sentence for Settings → Mods.
    pub summary: &'static str,
    pub version: Version,
    pub api_version: u32,
    /// The oldest Yappuccino the mod runs in.
    pub min_app_version: Version,
    pub permissions: &'static [Permission],
    pub inspiration: Option<Inspiration>,
    /// What Settings → Mods offers to change while the mod is on.
    pub settings: &'static [Setting],
}

/// One of a mod's settings, kept in `layout.json` by its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: SettingKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// A line of text, at most [`TEXT_LIMIT`] characters, empty unless set.
    Text {
        placeholder: &'static str,
    },
    Switch {
        default: bool,
    },
}

/// The longest text setting, in characters.
pub const TEXT_LIMIT: usize = 500;

/// Whether a mod can run in this Yappuccino.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    /// It needs at least this version of Yappuccino.
    NeedsApp(Version),
    /// It was written for another host API.
    Api(u32),
}

impl Manifest {
    pub fn compatibility(&self, app: Version) -> Compatibility {
        if self.api_version != API_VERSION {
            Compatibility::Api(self.api_version)
        } else if app < self.min_app_version {
            Compatibility::NeedsApp(self.min_app_version)
        } else {
            Compatibility::Compatible
        }
    }
}

/// Message Utilities: Copy Message Link and Copy Message ID in a message's
/// menu.
pub const MESSAGE_UTILITIES: &str = "message-utilities";

/// Notification Tweaks: muted words and quiet hours for message notifications.
pub const NOTIFICATION_TWEAKS: &str = "notification-tweaks";

/// Privacy Shield: pictures and videos covered until clicked.
pub const PRIVACY_SHIELD: &str = "privacy-shield";

/// Compact Mode: less space between groups of messages.
pub const COMPACT_MODE: &str = "compact-mode";

/// Every mod Yappuccino ships, in the order Settings → Mods lists them.
pub const CATALOG: &[Manifest] = &[
    Manifest {
        id: MESSAGE_UTILITIES,
        name: "Message Utilities",
        summary: "Copy a message's link or ID from its menu, without turning on Developer Mode.",
        version: Version::new(1, 0, 0),
        api_version: API_VERSION,
        min_app_version: Version::new(0, 3, 0),
        permissions: &[Permission::MessageMenu, Permission::Clipboard],
        inspiration: Some(Inspiration {
            name: "Discord's own message menu",
            licence: "proprietary; behaviour only",
        }),
        settings: &[],
    },
    Manifest {
        id: NOTIFICATION_TWEAKS,
        name: "Notification Tweaks",
        summary: "Mute message notifications that contain certain words, and keep quiet hours. Calls still ring.",
        version: Version::new(1, 0, 0),
        api_version: API_VERSION,
        min_app_version: Version::new(0, 3, 0),
        permissions: &[Permission::Notifications],
        inspiration: Some(Inspiration {
            name: "Vencord's notification plugins",
            licence: "GPL-3.0; behaviour only",
        }),
        settings: &[
            Setting {
                key: "muted_words",
                label: "Muted words",
                description: "A message containing any of these, separated by commas, makes no sound or notification. Case doesn't matter.",
                kind: SettingKind::Text {
                    placeholder: "spoilers, giveaway",
                },
            },
            Setting {
                key: "quiet_hours",
                label: "Quiet hours",
                description: "From and to, on a 24-hour clock, such as 22:00-07:00. Messages make no sound or notification in between.",
                kind: SettingKind::Text {
                    placeholder: "22:00-07:00",
                },
            },
            Setting {
                key: "mentions_through",
                label: "Mentions come through",
                description: "A message that mentions you notifies anyway.",
                kind: SettingKind::Switch { default: true },
            },
        ],
    },
    Manifest {
        id: PRIVACY_SHIELD,
        name: "Privacy Shield",
        summary: "Covers every picture and video in messages, as a spoiler is, until you click it.",
        version: Version::new(1, 0, 0),
        api_version: API_VERSION,
        min_app_version: Version::new(0, 3, 0),
        permissions: &[Permission::MediaDisplay],
        inspiration: Some(Inspiration {
            name: "Discord's spoilers and Vencord's privacy plugins",
            licence: "proprietary, GPL-3.0; behaviour only",
        }),
        settings: &[],
    },
    Manifest {
        id: COMPACT_MODE,
        name: "Compact Mode",
        summary: "Fits more of a conversation on screen: one author's messages and the next author's sit closer together.",
        version: Version::new(1, 0, 0),
        api_version: API_VERSION,
        min_app_version: Version::new(0, 3, 0),
        permissions: &[Permission::MessageLayout],
        inspiration: Some(Inspiration {
            name: "Discord's compact message display",
            licence: "proprietary; behaviour only",
        }),
        settings: &[],
    },
];

/// A mod by its id.
pub fn manifest(id: &str) -> Option<&'static Manifest> {
    CATALOG.iter().find(|manifest| manifest.id == id)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModError {
    Unknown(String),
    Incompatible(&'static str, Compatibility),
    /// A value that doesn't fit the setting, by the setting's label.
    Invalid(&'static str),
}

impl fmt::Display for ModError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(formatter, "no mod is called {id}"),
            Self::Incompatible(id, Compatibility::NeedsApp(version)) => {
                write!(formatter, "{id} needs Yappuccino {version} or later")
            }
            Self::Incompatible(id, Compatibility::Api(api)) => {
                write!(
                    formatter,
                    "{id} was written for mod API {api}, not {API_VERSION}"
                )
            }
            Self::Incompatible(id, Compatibility::Compatible) => {
                write!(formatter, "{id} could not be turned on")
            }
            Self::Invalid(label) => write!(formatter, "{label} doesn't take that value"),
        }
    }
}

impl std::error::Error for ModError {}

/// Which mods are on, and each one's settings, as `layout.json` keeps them.
/// A mod that fails while it runs stays off until Yappuccino starts again,
/// without losing its saved settings.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Mods {
    enabled: BTreeSet<String>,
    /// Each mod's own settings, by mod id then setting name; kept when the mod
    /// is turned off, so turning it on again finds them.
    settings: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    #[serde(skip)]
    failed: BTreeSet<&'static str>,
    /// This Yappuccino's version, which the host sets once it has read the file.
    #[serde(skip)]
    app: Option<Version>,
}

impl Mods {
    /// Sets the running Yappuccino's version, which [`Self::set_enabled`]
    /// checks manifests against.
    pub fn for_app(&mut self, app: Version) {
        self.app = Some(app);
    }

    /// Whether a mod is on and running. Ids from an older Yappuccino that no
    /// longer ships them are kept but never count.
    pub fn enabled(&self, id: &str) -> bool {
        self.enabled.contains(id)
            && manifest(id).is_some_and(|manifest| {
                !self.failed.contains(manifest.id)
                    && self
                        .app
                        .is_none_or(|app| manifest.compatibility(app) == Compatibility::Compatible)
            })
    }

    /// Whether the mod is on and its manifest grants the permission: the
    /// one question the host asks before a mod acts.
    pub fn allows(&self, id: &str, permission: Permission) -> bool {
        self.enabled(id)
            && manifest(id).is_some_and(|manifest| manifest.permissions.contains(&permission))
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> Result<(), ModError> {
        let manifest = manifest(id).ok_or_else(|| ModError::Unknown(id.to_owned()))?;
        if on {
            if let Some(app) = self.app {
                match manifest.compatibility(app) {
                    Compatibility::Compatible => {}
                    problem => return Err(ModError::Incompatible(manifest.id, problem)),
                }
            }
            self.failed.remove(manifest.id);
            self.enabled.insert(manifest.id.to_owned());
        } else {
            self.enabled.remove(manifest.id);
        }
        Ok(())
    }

    /// Turns a mod off for the rest of this run after it failed, so one
    /// broken mod can't keep breaking the app; it stays on in `layout.json`.
    pub fn fail(&mut self, id: &str, error: &dyn fmt::Display) {
        if let Some(manifest) = manifest(id) {
            tracing::warn!(id, %error, "mod failed; off until Yappuccino restarts");
            self.failed.insert(manifest.id);
        }
    }

    pub fn failed(&self, id: &str) -> bool {
        self.failed.contains(id)
    }

    /// One of a mod's settings, if it was set.
    pub fn setting(&self, id: &str, name: &str) -> Option<&serde_json::Value> {
        self.settings.get(id)?.get(name)
    }

    /// A text setting: empty unless set.
    pub fn text(&self, id: &str, key: &str) -> &str {
        self.setting(id, key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    }

    /// A switch: its manifest's default unless set.
    pub fn switch(&self, id: &str, key: &str) -> bool {
        self.setting(id, key)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| {
                spec(id, key).is_some_and(|setting| {
                    matches!(setting.kind, SettingKind::Switch { default: true })
                })
            })
    }

    /// Sets one of a mod's settings, as its manifest declares it: a text up to
    /// [`TEXT_LIMIT`] characters on one line, or a switch.
    pub fn set_setting(
        &mut self,
        id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), ModError> {
        let setting = spec(id, key).ok_or_else(|| ModError::Unknown(format!("{id}/{key}")))?;
        let fits = match (setting.kind, &value) {
            (SettingKind::Text { .. }, serde_json::Value::String(text)) => {
                text.chars().count() <= TEXT_LIMIT
                    && !text.chars().any(char::is_control)
                    // Quiet hours are a span or nothing, never a typo kept as one.
                    && (key != "quiet_hours" || text.trim().is_empty() || quiet_hours(text).is_some())
            }
            (SettingKind::Switch { .. }, serde_json::Value::Bool(_)) => true,
            _ => false,
        };
        if !fits {
            return Err(ModError::Invalid(setting.label));
        }
        self.settings
            .entry(id.to_owned())
            .or_default()
            .insert(key.to_owned(), value);
        Ok(())
    }
}

fn spec(id: &str, key: &str) -> Option<&'static Setting> {
    manifest(id)?
        .settings
        .iter()
        .find(|setting| setting.key == key)
}

/// What Notification Tweaks silences, read once per message from its
/// settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quiet {
    /// Lowercase.
    words: Vec<String>,
    /// From and to, in minutes after midnight; may wrap past midnight.
    hours: Option<(u16, u16)>,
    mentions_through: bool,
}

impl Quiet {
    /// Notification Tweaks' rules, if it's on.
    pub fn of(mods: &Mods) -> Option<Self> {
        if !mods.allows(NOTIFICATION_TWEAKS, Permission::Notifications) {
            return None;
        }
        Some(Self {
            words: mods
                .text(NOTIFICATION_TWEAKS, "muted_words")
                .split(',')
                .map(|word| word.trim().to_lowercase())
                .filter(|word| !word.is_empty())
                .collect(),
            hours: quiet_hours(mods.text(NOTIFICATION_TWEAKS, "quiet_hours")),
            mentions_through: mods.switch(NOTIFICATION_TWEAKS, "mentions_through"),
        })
    }

    /// Whether a message, by its title and text, keeps quiet at `minute`
    /// minutes after local midnight.
    pub fn silences(&self, title: &str, body: &str, mention: bool, minute: u16) -> bool {
        if mention && self.mentions_through {
            return false;
        }
        let quiet_now = self.hours.is_some_and(|(from, to)| {
            if from <= to {
                (from..to).contains(&minute)
            } else {
                minute >= from || minute < to
            }
        });
        quiet_now || {
            let (title, body) = (title.to_lowercase(), body.to_lowercase());
            self.words
                .iter()
                .any(|word| body.contains(word.as_str()) || title.contains(word.as_str()))
        }
    }
}

/// Reads `22:00-07:00` as minutes after midnight; `None` for anything else,
/// or for a span that starts and ends at the same minute.
pub fn quiet_hours(text: &str) -> Option<(u16, u16)> {
    let minute = |time: &str| {
        let (hour, minute) = time.trim().split_once(':')?;
        let (hour, minute) = (hour.parse::<u16>().ok()?, minute.parse::<u16>().ok()?);
        (hour < 24 && minute < 60).then_some(hour * 60 + minute)
    };
    let (from, to) = text.split_once('-')?;
    let (from, to) = (minute(from)?, minute(to)?);
    (from != to).then_some((from, to))
}

/// Minutes after midnight, local time.
pub fn local_minute() -> u16 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    (now.hour() * 60 + now.minute()) as u16
}

/// A link to a sent message that its network's apps open: Discord's own, or
/// matrix.to's for a Matrix event. `None` for a message still sending.
pub fn message_link(room: &str, item: &str) -> Option<String> {
    if let Some(id) = item.strip_prefix("d:") {
        let channel = room.strip_prefix(crate::discord::PREFIX)?;
        return (!id.is_empty()).then(|| {
            format!(
                "https://discord.com/channels/{}/{id}",
                channel.replacen(':', "/", 1)
            )
        });
    }
    let event = item.strip_prefix("e:")?;
    Some(format!("https://matrix.to/#/{room}/{event}"))
}

/// A sent message's id as its network knows it.
pub fn message_id(item: &str) -> Option<&str> {
    item.strip_prefix("d:")
        .or_else(|| item.strip_prefix("e:"))
        .filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: Version = Version::new(0, 3, 0);

    fn mods() -> Mods {
        let mut mods = Mods::default();
        mods.for_app(APP);
        mods
    }

    #[test]
    fn the_catalog_is_well_formed() {
        let mut ids = BTreeSet::new();
        for manifest in CATALOG {
            assert!(ids.insert(manifest.id), "{} is listed twice", manifest.id);
            assert!(
                manifest
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} isn't lowercase with dashes",
                manifest.id
            );
            assert_eq!(manifest.api_version, API_VERSION, "{}", manifest.id);
            assert!(
                !manifest.permissions.is_empty(),
                "{} asks for nothing",
                manifest.id
            );
            assert_eq!(manifest.compatibility(APP), Compatibility::Compatible);
        }
    }

    #[test]
    fn mods_are_off_until_turned_on() {
        let mut mods = mods();
        assert!(!mods.enabled(MESSAGE_UTILITIES));
        assert!(!mods.allows(MESSAGE_UTILITIES, Permission::Clipboard));
        mods.set_enabled(MESSAGE_UTILITIES, true).unwrap();
        assert!(mods.allows(MESSAGE_UTILITIES, Permission::MessageMenu));
        assert!(mods.allows(MESSAGE_UTILITIES, Permission::Clipboard));
        mods.set_enabled(MESSAGE_UTILITIES, false).unwrap();
        assert!(!mods.enabled(MESSAGE_UTILITIES));
    }

    #[test]
    fn unknown_and_incompatible_mods_stay_off() {
        let mut mods = mods();
        assert_eq!(
            mods.set_enabled("nope", true),
            Err(ModError::Unknown("nope".into()))
        );
        let mut old = Mods::default();
        old.for_app(Version::new(0, 2, 9));
        assert_eq!(
            old.set_enabled(MESSAGE_UTILITIES, true),
            Err(ModError::Incompatible(
                MESSAGE_UTILITIES,
                Compatibility::NeedsApp(Version::new(0, 3, 0))
            ))
        );
        // One saved by a newer Yappuccino doesn't run in this older one.
        let saved: Mods =
            serde_json::from_str(r#"{"enabled":["message-utilities","gone"]}"#).unwrap();
        let mut older = saved.clone();
        older.for_app(Version::new(0, 2, 0));
        assert!(!older.enabled(MESSAGE_UTILITIES));
        assert!(!older.enabled("gone"));
        mods = saved;
        mods.for_app(APP);
        assert!(mods.enabled(MESSAGE_UTILITIES));
        assert_eq!(
            mods.set_enabled("gone", false),
            Err(ModError::Unknown("gone".into()))
        );
    }

    #[test]
    fn a_failed_mod_is_off_for_the_run_but_kept() {
        let mut mods = mods();
        mods.set_enabled(MESSAGE_UTILITIES, true).unwrap();
        mods.set_setting(NOTIFICATION_TWEAKS, "muted_words", "spoilers".into())
            .unwrap();
        mods.fail(MESSAGE_UTILITIES, &"broke");
        assert!(!mods.enabled(MESSAGE_UTILITIES));
        assert!(mods.failed(MESSAGE_UTILITIES));
        let saved = serde_json::to_string(&mods).unwrap();
        let mut again: Mods = serde_json::from_str(&saved).unwrap();
        again.for_app(APP);
        assert!(again.enabled(MESSAGE_UTILITIES), "on again after a restart");
        assert_eq!(again.text(NOTIFICATION_TWEAKS, "muted_words"), "spoilers");
        // Turning it on again in the same run clears the failure.
        mods.set_enabled(MESSAGE_UTILITIES, true).unwrap();
        assert!(mods.enabled(MESSAGE_UTILITIES));
    }

    #[test]
    fn versions_read_with_or_without_a_suffix() {
        assert_eq!(Version::parse("0.4.0"), Some(Version::new(0, 4, 0)));
        assert_eq!(
            Version::parse("0.4.0-nightly.20261109.1"),
            Some(Version::new(0, 4, 0))
        );
        assert_eq!(Version::parse("0.4"), None);
        assert_eq!(Version::parse("0.4.0.1"), None);
        assert_eq!(Version::parse("x.4.0"), None);
        assert!(Version::new(0, 10, 0) > Version::new(0, 9, 9));
    }

    #[test]
    fn settings_take_only_what_their_manifest_declares() {
        let mut mods = mods();
        assert_eq!(mods.text(NOTIFICATION_TWEAKS, "quiet_hours"), "");
        assert!(
            mods.switch(NOTIFICATION_TWEAKS, "mentions_through"),
            "on by default"
        );
        mods.set_setting(NOTIFICATION_TWEAKS, "mentions_through", false.into())
            .unwrap();
        assert!(!mods.switch(NOTIFICATION_TWEAKS, "mentions_through"));
        assert_eq!(
            mods.set_setting(NOTIFICATION_TWEAKS, "quiet_hours", true.into()),
            Err(ModError::Invalid("Quiet hours"))
        );
        assert_eq!(
            mods.set_setting(NOTIFICATION_TWEAKS, "muted_words", "a\nb".into()),
            Err(ModError::Invalid("Muted words"))
        );
        assert_eq!(
            mods.set_setting(
                NOTIFICATION_TWEAKS,
                "muted_words",
                "x".repeat(TEXT_LIMIT + 1).into()
            ),
            Err(ModError::Invalid("Muted words"))
        );
        assert!(
            mods.set_setting(NOTIFICATION_TWEAKS, "nope", "x".into())
                .is_err()
        );
        assert!(mods.set_setting("nope", "muted_words", "x".into()).is_err());
        // Quiet hours are a span, or empty to turn them off.
        assert_eq!(
            mods.set_setting(NOTIFICATION_TWEAKS, "quiet_hours", "10pm".into()),
            Err(ModError::Invalid("Quiet hours"))
        );
        assert!(
            mods.set_setting(NOTIFICATION_TWEAKS, "quiet_hours", "".into())
                .is_ok()
        );
    }

    #[test]
    fn quiet_hours_read_from_and_to() {
        assert_eq!(quiet_hours("22:00-07:00"), Some((22 * 60, 7 * 60)));
        assert_eq!(
            quiet_hours(" 9:30 - 17:05 "),
            Some((9 * 60 + 30, 17 * 60 + 5))
        );
        assert_eq!(quiet_hours("24:00-07:00"), None);
        assert_eq!(quiet_hours("22:60-07:00"), None);
        assert_eq!(quiet_hours("22:00"), None);
        assert_eq!(quiet_hours("08:00-08:00"), None);
        assert_eq!(quiet_hours(""), None);
    }

    #[test]
    fn notification_tweaks_silence_words_and_quiet_hours() {
        let mut mods = mods();
        assert_eq!(Quiet::of(&mods), None, "off until turned on");
        mods.set_enabled(NOTIFICATION_TWEAKS, true).unwrap();
        mods.set_setting(
            NOTIFICATION_TWEAKS,
            "muted_words",
            " Giveaway, ,spoilers ".into(),
        )
        .unwrap();
        mods.set_setting(NOTIFICATION_TWEAKS, "quiet_hours", "22:00-07:00".into())
            .unwrap();
        let quiet = Quiet::of(&mods).unwrap();
        let noon = 12 * 60;
        assert!(quiet.silences("#general", "A GIVEAWAY!", false, noon));
        assert!(quiet.silences("Spoilers chat", "hi", false, noon));
        assert!(!quiet.silences("#general", "hello", false, noon));
        // Across midnight.
        assert!(quiet.silences("#general", "hello", false, 23 * 60));
        assert!(quiet.silences("#general", "hello", false, 3 * 60));
        assert!(!quiet.silences("#general", "hello", false, 7 * 60));
        // A mention comes through, unless that's turned off.
        assert!(!quiet.silences("#general", "giveaway", true, 23 * 60));
        mods.set_setting(NOTIFICATION_TWEAKS, "mentions_through", false.into())
            .unwrap();
        assert!(
            Quiet::of(&mods)
                .unwrap()
                .silences("#general", "hi", true, 23 * 60)
        );
        // Hours within one day.
        mods.set_setting(NOTIFICATION_TWEAKS, "quiet_hours", "09:00-17:00".into())
            .unwrap();
        let quiet = Quiet::of(&mods).unwrap();
        assert!(quiet.silences("x", "y", false, 9 * 60));
        assert!(!quiet.silences("x", "y", false, 17 * 60));
        assert!(local_minute() < 24 * 60);
    }

    #[test]
    fn links_name_the_message_on_its_network() {
        assert_eq!(
            message_link("discord:10:21", "d:99").as_deref(),
            Some("https://discord.com/channels/10/21/99")
        );
        assert_eq!(
            message_link("discord:@me:30", "d:99").as_deref(),
            Some("https://discord.com/channels/@me/30/99")
        );
        assert_eq!(
            message_link("!room:example.org", "e:$event").as_deref(),
            Some("https://matrix.to/#/!room:example.org/$event")
        );
        assert_eq!(message_link("!room:example.org", "t:pending"), None);
        assert_eq!(message_link("discord:10:21", "d:"), None);
        assert_eq!(message_id("d:99"), Some("99"));
        assert_eq!(message_id("e:$event"), Some("$event"));
        assert_eq!(message_id("t:pending"), None);
    }
}
