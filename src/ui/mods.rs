//! Settings → Mods, and what the mods add to the window: each asks
//! [`Mods::allows`] before it acts. See `docs/plugins.md`.

use yappuccino_core::chat::Alert;
use yappuccino_core::mods::{
    self, CATALOG, MESSAGE_UTILITIES, Mods, PRIVACY_SHIELD, Permission, Quiet, SettingKind,
};

use super::*;

pub(super) fn bind(window: &YappuccinoWindow, adapter: &Rc<RefCell<Adapter>>) {
    let state = adapter.clone();
    let weak = window.as_weak();
    window.on_mod_toggled(move |id, on| {
        let Some(window) = weak.upgrade() else { return };
        let mut adapter = state.borrow_mut();
        if let Err(error) = adapter.layout.mods.set_enabled(&id, on) {
            tracing::warn!(%error, "could not change a mod");
        }
        adapter.layout.write(&adapter.root);
        show(&window, &adapter.layout.mods);
        // Message rows show what the mods change, such as covered pictures.
        adapter.restyle = true;
        adapter.refresh(&window);
    });
    let state = adapter.clone();
    let weak = window.as_weak();
    window.on_mod_text_saved(move |id, key, text| {
        let mut adapter = state.borrow_mut();
        let saved = adapter
            .layout
            .mods
            .set_setting(&id, &key, text.trim().into());
        if let Err(error) = &saved {
            tracing::debug!(%error, "a mod's setting wasn't saved");
            return false;
        }
        adapter.layout.write(&adapter.root);
        if let Some(window) = weak.upgrade() {
            show(&window, &adapter.layout.mods);
        }
        true
    });
    let state = adapter.clone();
    let weak = window.as_weak();
    window.on_mod_switch_changed(move |id, key, on| {
        let mut adapter = state.borrow_mut();
        if let Err(error) = adapter.layout.mods.set_setting(&id, &key, on.into()) {
            tracing::warn!(%error, "could not change a mod's setting");
        }
        adapter.layout.write(&adapter.root);
        if let Some(window) = weak.upgrade() {
            show(&window, &adapter.layout.mods);
        }
    });
    let state = adapter.clone();
    let weak = window.as_weak();
    window.on_copy_message_link(move |item| {
        let Some(window) = weak.upgrade() else { return };
        let adapter = state.borrow();
        if !adapter
            .layout
            .mods
            .allows(MESSAGE_UTILITIES, Permission::Clipboard)
        {
            return;
        }
        let state = &adapter.app.state;
        match room_of(state, &item).and_then(|room| mods::message_link(room, &item)) {
            Some(link) => window.invoke_copy_text(link.into()),
            None => tracing::debug!(%item, "no link for this message"),
        }
    });
    let state = adapter.clone();
    let weak = window.as_weak();
    window.on_copy_message_id(move |item| {
        let Some(window) = weak.upgrade() else { return };
        if !state
            .borrow()
            .layout
            .mods
            .allows(MESSAGE_UTILITIES, Permission::Clipboard)
        {
            return;
        }
        if let Some(id) = mods::message_id(&item) {
            window.invoke_copy_text(id.into());
        }
    });
}

/// Shows the catalog in Settings, and turns on what each mod adds.
pub(super) fn show(window: &YappuccinoWindow, mods: &Mods) {
    let rows: Vec<ModRow> = CATALOG
        .iter()
        .map(|manifest| ModRow {
            id: manifest.id.into(),
            name: manifest.name.into(),
            version: manifest.version.to_string().into(),
            summary: manifest.summary.into(),
            permissions: manifest
                .permissions
                .iter()
                .map(|permission| permission.describe())
                .collect::<Vec<_>>()
                .join(" · ")
                .into(),
            inspiration: manifest
                .inspiration
                .map(|inspiration| {
                    format!(
                        "Behaviour from {} ({})",
                        inspiration.name, inspiration.licence
                    )
                })
                .unwrap_or_default()
                .into(),
            enabled: mods.enabled(manifest.id),
            failed: mods.failed(manifest.id),
            settings: ModelRc::new(VecModel::from(
                manifest
                    .settings
                    .iter()
                    .map(|setting| {
                        let (text, placeholder) = match setting.kind {
                            SettingKind::Text { placeholder } => (true, placeholder),
                            SettingKind::Switch { .. } => (false, ""),
                        };
                        ModSetting {
                            key: setting.key.into(),
                            label: setting.label.into(),
                            description: setting.description.into(),
                            text,
                            placeholder: placeholder.into(),
                            value: mods.text(manifest.id, setting.key).into(),
                            on: !text && mods.switch(manifest.id, setting.key),
                        }
                    })
                    .collect::<Vec<_>>(),
            )),
        })
        .collect();
    window.set_mods(ModelRc::new(VecModel::from(rows)));
    window.set_message_utilities(mods.allows(MESSAGE_UTILITIES, Permission::MessageMenu));
    pictures::set_cover_media(mods.allows(PRIVACY_SHIELD, Permission::MediaDisplay));
}

/// Notification Tweaks keeps a message quiet: no sound, taskbar flash or
/// desktop notification. A call always rings.
pub(super) fn silenced(mods: &Mods, alert: &Alert) -> bool {
    !alert.call
        && Quiet::of(mods).is_some_and(|quiet| {
            quiet.silences(
                &alert.title,
                &alert.body,
                alert.mention,
                mods::local_minute(),
            )
        })
}

/// The room that shows a message: the one selected, or the thread beside it.
fn room_of<'a>(state: &'a AppState, item: &str) -> Option<&'a str> {
    if let Some(aside) = &state.aside
        && aside.messages.iter().any(|message| message.item == item)
    {
        return Some(&aside.room);
    }
    state
        .messages
        .iter()
        .any(|message| message.item == item)
        .then_some(state.selected.as_deref())
        .flatten()
}
