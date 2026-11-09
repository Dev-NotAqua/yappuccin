//! Appearance: choosing a theme, and making and editing the person's own.

use super::*;

impl Adapter {
    /// Shows the window in a theme, built in or the person's own, and keeps
    /// it chosen. False for an id Yappuccino doesn't know, which changes nothing.
    pub(super) fn choose_theme(&mut self, window: &YappuccinoWindow, id: &str) -> bool {
        let Some((id, chosen)) =
            theme::lookup(id, &self.layout.custom_themes).map(|(id, theme)| (id.to_owned(), theme))
        else {
            return false;
        };
        window.set_theme_id(id.as_str().into());
        self.set_theme(window, &chosen);
        self.layout.theme = Some(id);
        self.layout.write(&self.root);
        true
    }

    /// Shows the window in another theme's colours, building again what
    /// carries the old mention colour in its markup: message rows, the
    /// topic, pins and search results.
    pub(super) fn set_theme(&mut self, window: &YappuccinoWindow, theme: &theme::Theme) {
        if theme.apply(window) {
            self.restyle_mentions();
            self.refresh(window);
        }
    }

    /// Marks for building again, at the next refresh, what carries the
    /// mention colour in its markup.
    pub(super) fn restyle_mentions(&mut self) {
        self.restyle = true;
        self.topic_shown = None;
        self.models.pins.clear();
        self.models.search_results.clear();
    }

    /// Opens the editor on a copy of the theme shown, which the person can
    /// swap for any other.
    pub(super) fn new_theme(&mut self, window: &YappuccinoWindow) {
        let current = window.get_theme_id();
        let starts: Vec<_> = theme::starts(&self.layout.custom_themes).collect();
        let index = starts
            .iter()
            .position(|start| start.id == current.as_str())
            .unwrap_or(0);
        let names: Vec<SharedString> = starts.iter().map(|start| start.name.into()).collect();
        let (base, start) = (starts[index].base, starts[index].theme);
        window.set_theme_name(copy_name(starts[index].name).into());
        window.set_theme_starts(ModelRc::new(VecModel::from(names)));
        window.set_theme_start(index as i32);
        let mut editor = theme::Editor::new(None, base, start);
        editor.background_image = starts[index].background_image.map(str::to_owned);
        self.open_theme_editor(window, editor);
    }

    /// Opens the editor on one of the person's own themes.
    pub(super) fn edit_theme(&mut self, window: &YappuccinoWindow, id: &str) {
        let Some(custom) = self
            .layout
            .custom_themes
            .iter()
            .find(|custom| custom.id == id)
        else {
            return;
        };
        window.set_theme_name(custom.name.as_str().into());
        let editor = theme::Editor::new(Some(custom.id.clone()), custom.base, custom.theme);
        let mut editor = editor;
        editor.background_image = custom.background_image.clone();
        self.open_theme_editor(window, editor);
    }

    fn open_theme_editor(&mut self, window: &YappuccinoWindow, editor: theme::Editor) {
        window.set_theme_editing_saved(editor.editing.is_some());
        window.set_theme_dark(editor.theme.dark);
        window.set_theme_colours(editor.rows());
        window.set_theme_can_save(true);
        window.set_theme_editor(true);
        window.set_theme_background_image(editor.background_image.clone().unwrap_or_default().into());
        self.set_theme(window, &editor.theme);
        self.theme_editor = Some(editor);
    }

    /// The new theme starts again from another, and takes its name too
    /// unless the person named it.
    pub(super) fn theme_start_chosen(&mut self, window: &YappuccinoWindow, index: i32) {
        let custom = &self.layout.custom_themes;
        let start = |index: i32| {
            let index = usize::try_from(index).ok()?;
            theme::starts(custom)
                .nth(index)
                .map(|start| (start.base, start.theme, copy_name(start.name)))
        };
        let (Some(editor), Some((base, theme, name))) = (self.theme_editor.as_mut(), start(index))
        else {
            return;
        };
        let named = start(window.get_theme_start())
            .is_none_or(|(_, _, before)| before != window.get_theme_name().as_str());
        if !named {
            window.set_theme_name(name.into());
        }
        editor.restart(base, theme);
        let bg = theme::starts(&self.layout.custom_themes)
            .nth(index as usize)
            .and_then(|s| s.background_image.map(str::to_owned));
        editor.background_image = bg;
        window.set_theme_start(index);
        window.set_theme_dark(theme.dark);
        window.set_theme_can_save(true);
        self.set_theme(window, &theme);
    }

    /// A colour's field changed: the window shows it at once if it's a
    /// colour.
    pub(super) fn theme_colour_edited(&mut self, window: &YappuccinoWindow, index: i32, text: &str) {
        let Some(editor) = self.theme_editor.as_mut() else {
            return;
        };
        let changed = usize::try_from(index).is_ok_and(|index| editor.edit(index, text));
        window.set_theme_can_save(editor.valid());
        if changed {
            let theme = editor.theme;
            self.set_theme(window, &theme);
        }
    }

    pub(super) fn theme_dark_toggled(&mut self, window: &YappuccinoWindow, dark: bool) {
        let Some(editor) = self.theme_editor.as_mut() else {
            return;
        };
        editor.theme.dark = dark;
        let theme = editor.theme;
        self.set_theme(window, &theme);
    }

    /// Keeps the theme as edited, among the person's own, and chooses it.
    pub(super) fn save_theme(&mut self, window: &YappuccinoWindow) {
        // Sync the background image field back into the editor.
        if let Some(editor) = self.theme_editor.as_mut() {
            let bg = window.get_theme_background_image();
            editor.background_image = if bg.is_empty() { None } else { Some(bg.to_string()) };
        }
        let Some(editor) = self.theme_editor.take_if(|editor| editor.valid()) else {
            return;
        };
        let name = match window.get_theme_name().trim() {
            "" => "My theme".to_owned(),
            name => name.to_owned(),
        };
        let custom = &mut self.layout.custom_themes;
        let saved = editor
            .editing
            .as_ref()
            .and_then(|id| custom.iter_mut().find(|custom| custom.id == *id));
        let id = match saved {
            Some(saved) => {
                saved.name = name;
                saved.theme = editor.theme;
                saved.background_image = editor.background_image.clone();
                saved.id.clone()
            }
            None => {
                let id = theme::new_id(custom);
                custom.push(theme::Custom {
                    id: id.clone(),
                    name,
                    base: editor.base,
                    theme: editor.theme,
                    background_image: editor.background_image.clone(),
                });
                id
            }
        };
        close_theme_editor(window);
        window.set_themes(theme::cards(&self.layout.custom_themes));
        self.choose_theme(window, &id);
    }

    /// Closes the editor and shows the theme from before it opened.
    pub(super) fn cancel_theme(&mut self, window: &YappuccinoWindow) {
        if self.theme_editor.take().is_some() {
            close_theme_editor(window);
            let (_, before) =
                theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes);
            self.set_theme(window, &before);
        }
    }

    /// Deletes the theme being edited. If it was the chosen one, Yap shows.
    pub(super) fn delete_theme(&mut self, window: &YappuccinoWindow) {
        let Some(editor) = self.theme_editor.take() else {
            return;
        };
        close_theme_editor(window);
        if let Some(id) = editor.editing {
            self.layout.custom_themes.retain(|custom| custom.id != id);
        }
        window.set_themes(theme::cards(&self.layout.custom_themes));
        let (id, _) = theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes);
        let id = id.to_owned();
        self.choose_theme(window, &id);
    }

    /// Exports the theme being edited (or the chosen one) to a JSON file.
    pub(super) fn theme_export(&mut self, window: &YappuccinoWindow) {
        #[cfg(feature = "desktop")]
        {
            let theme = self
                .theme_editor
                .as_ref()
                .map(|e| e.theme)
                .or_else(|| {
                    let (id, t) = theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes);
                    let _ = id;
                    Some(t)
                })
                .unwrap_or_else(|| theme::YAP);
            let name = if self.theme_editor.is_some() {
                window.get_theme_name().to_string()
            } else {
                theme::chosen(self.layout.theme.as_deref(), &self.layout.custom_themes)
                    .0
                    .to_owned()
            };
            let custom = theme::Custom {
                id: "exported".into(),
                name,
                base: "yap",
                theme,
                background_image: self
                    .theme_editor
                    .as_ref()
                    .and_then(|e| e.background_image.clone()),
            };
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("theme.json")
                .add_filter("Theme", &["json"])
                .save_file()
            {
                if let Ok(text) = serde_json::to_string_pretty(&custom) {
                    let _ = std::fs::write(path, text);
                }
            }
        }
    }

    /// Imports a theme from a JSON file and selects it.
    pub(super) fn theme_import(&mut self, window: &YappuccinoWindow) {
        #[cfg(feature = "desktop")]
        {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Theme", &["json"])
                .pick_file()
            else {
                return;
            };
            let Ok(text) = std::fs::read_to_string(path) else {
                return;
            };
            let Ok(mut custom): Result<theme::Custom, _> = serde_json::from_str(&text) else {
                return;
            };
            // Give it a fresh id so it never collides.
            custom.id = theme::new_id(&self.layout.custom_themes);
            if custom.name.trim().is_empty() {
                custom.name = "Imported theme".into();
            }
            self.layout.custom_themes.push(custom.clone());
            self.layout.write(&self.root);
            window.set_themes(theme::cards(&self.layout.custom_themes));
            self.choose_theme(window, &custom.id);
        }
    }

    /// Lets the person pick a background image for the theme being edited.
    pub(super) fn theme_pick_background(&mut self, window: &YappuccinoWindow) {
        #[cfg(feature = "desktop")]
        {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
                .pick_file()
            else {
                return;
            };
            if let Some(editor) = self.theme_editor.as_mut() {
                editor.background_image = Some(path.to_string_lossy().into_owned());
            }
            window.set_theme_background_image(path.to_string_lossy().to_string().into());
        }
    }
}

/// What a new theme copied from another is called until the person names it.
fn copy_name(name: &str) -> String {
    format!("{name} copy")
}

/// Drops the editor's models from the window, so they're held only while
/// it's open.
fn close_theme_editor(window: &YappuccinoWindow) {
    window.set_theme_editor(false);
    window.set_theme_colours(ModelRc::default());
    window.set_theme_starts(ModelRc::default());
}
