use crate::app_state::TradingTerminal;
use crate::config;

// ---------------------------------------------------------------------------
// Layout Names And Hotkeys
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn update_saved_layout_snapshot(&mut self, name: String) {
        let new_layout = self.saved_layout_snapshot(name.clone());
        if let Some(pos) = self
            .saved_layouts
            .iter()
            .position(|layout| layout.name == name)
        {
            self.saved_layouts[pos] = new_layout;
        } else {
            self.saved_layouts.push(new_layout);
        }
        self.active_layout_name = Some(name);
    }

    pub(super) fn reconcile_layout_rename_after_delete(&mut self, removed_index: Option<usize>) {
        let Some(removed_index) = removed_index else {
            return;
        };
        match self.layout_rename_index {
            Some(index) if index == removed_index => {
                self.layout_rename_index = None;
                self.layout_rename_input.clear();
            }
            Some(index) if index > removed_index => {
                self.layout_rename_index = Some(index - 1);
            }
            _ => {}
        }
    }

    pub(super) fn rename_saved_layout(&mut self, index: usize) {
        let Some(new_name) = normalized_layout_name(&self.layout_rename_input) else {
            self.push_toast("Layout name cannot be empty".to_string(), true);
            return;
        };
        let Some(layout) = self.saved_layouts.get(index) else {
            self.layout_rename_index = None;
            self.layout_rename_input.clear();
            return;
        };
        let old_name = layout.name.clone();
        if self
            .saved_layouts
            .iter()
            .enumerate()
            .any(|(pos, layout)| pos != index && layout.name == new_name)
        {
            self.push_toast(format!("Layout '{}' already exists", new_name), true);
            return;
        }

        self.saved_layouts[index].name = new_name.clone();
        if self.active_layout_name.as_deref() == Some(old_name.as_str()) {
            self.active_layout_name = Some(new_name.clone());
        }
        self.rename_layout_hotkeys(&old_name, &new_name);
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.persist_config();
    }

    pub(super) fn remove_layout_hotkeys(&mut self, name: &str) {
        self.hotkeys.retain(|hotkey| match &hotkey.action {
            config::HotkeyAction::SwitchLayout { name: layout_name } => layout_name != name,
            _ => true,
        });
        if self.recording_hotkey_for.as_ref().is_some_and(|action| {
            matches!(
                action,
                config::HotkeyAction::SwitchLayout { name: layout_name }
                    if layout_name == name
            )
        }) {
            self.recording_hotkey_for = None;
        }
    }

    fn rename_layout_hotkeys(&mut self, old_name: &str, new_name: &str) {
        for hotkey in &mut self.hotkeys {
            if let config::HotkeyAction::SwitchLayout { name } = &mut hotkey.action
                && name == old_name
            {
                *name = new_name.to_string();
            }
        }
        if let Some(config::HotkeyAction::SwitchLayout { name }) = &mut self.recording_hotkey_for
            && name == old_name
        {
            *name = new_name.to_string();
        }
    }
}

pub(super) fn normalized_layout_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
