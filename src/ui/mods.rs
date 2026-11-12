//! Settings → Mods, and what the mods add to the window: each asks
//! [`Mods::allows`] before it acts. See `docs/plugins.md`.

use yappuccino_core::mods::{self, CATALOG, MESSAGE_UTILITIES, Mods, Permission};

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
        })
        .collect();
    window.set_mods(ModelRc::new(VecModel::from(rows)));
    window.set_message_utilities(mods.allows(MESSAGE_UTILITIES, Permission::MessageMenu));
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
