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
}

impl Permission {
    /// What Settings → Mods says the permission lets the mod do.
    pub fn describe(self) -> &'static str {
        match self {
            Self::MessageMenu => "Add items to message menus",
            Self::Clipboard => "Copy text to the clipboard when you pick one",
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
}

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

/// Every mod Yappuccino ships, in the order Settings → Mods lists them.
pub const CATALOG: &[Manifest] = &[Manifest {
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
}];

/// A mod by its id.
pub fn manifest(id: &str) -> Option<&'static Manifest> {
    CATALOG.iter().find(|manifest| manifest.id == id)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModError {
    Unknown(String),
    Incompatible(&'static str, Compatibility),
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

    pub fn set_setting(&mut self, id: &str, name: &str, value: serde_json::Value) {
        self.settings
            .entry(id.to_owned())
            .or_default()
            .insert(name.to_owned(), value);
    }
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
        mods.set_setting(MESSAGE_UTILITIES, "format", "markdown".into());
        mods.fail(MESSAGE_UTILITIES, &"broke");
        assert!(!mods.enabled(MESSAGE_UTILITIES));
        assert!(mods.failed(MESSAGE_UTILITIES));
        let saved = serde_json::to_string(&mods).unwrap();
        let mut again: Mods = serde_json::from_str(&saved).unwrap();
        again.for_app(APP);
        assert!(again.enabled(MESSAGE_UTILITIES), "on again after a restart");
        assert_eq!(
            again.setting(MESSAGE_UTILITIES, "format"),
            Some(&serde_json::Value::from("markdown"))
        );
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
