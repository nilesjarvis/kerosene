use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_saved_layouts(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LayoutInputChanged(value) => {
                self.layout_input = value;
            }
            Message::SaveLayout(name) if !name.trim().is_empty() => {
                let new_layout = self.saved_layout_snapshot(name.clone());

                if let Some(pos) = self.saved_layouts.iter().position(|l| l.name == name) {
                    self.saved_layouts[pos] = new_layout;
                } else {
                    self.saved_layouts.push(new_layout);
                }
                self.active_layout_name = Some(name);
                self.layout_input.clear();
                self.persist_config();
            }
            Message::UpdateActiveLayout => {
                let Some(name) = self
                    .active_layout_name
                    .as_deref()
                    .and_then(normalized_layout_name)
                else {
                    self.push_toast("Select a saved layout before updating".to_string(), true);
                    return Task::none();
                };
                self.update_saved_layout_snapshot(name);
                self.layout_rename_index = None;
                self.layout_rename_input.clear();
                self.persist_config();
            }
            Message::LoadLayout(layout) => {
                self.close_chart_header_menus();
                self.active_layout_name = Some(layout.name.clone());
                let task = self.apply_layout(layout);
                self.persist_config();
                return task;
            }
            Message::DeleteLayout(name) => {
                let removed_index = self
                    .saved_layouts
                    .iter()
                    .position(|layout| layout.name == name);
                self.saved_layouts.retain(|layout| layout.name != name);
                if self.active_layout_name.as_deref() == Some(name.as_str()) {
                    self.active_layout_name = None;
                }
                self.reconcile_layout_rename_after_delete(removed_index);
                self.persist_config();
            }
            Message::LayoutRenameToggled(index) => {
                if self.layout_rename_index == Some(index) {
                    self.layout_rename_index = None;
                    self.layout_rename_input.clear();
                } else if let Some(layout) = self.saved_layouts.get(index) {
                    self.layout_rename_index = Some(index);
                    self.layout_rename_input = layout.name.clone();
                }
            }
            Message::LayoutRenameChanged(value) => {
                self.layout_rename_input = value;
            }
            Message::LayoutRenameSubmitted(index) => {
                self.rename_saved_layout(index);
            }
            Message::ExportLayout(layout) => {
                return Task::perform(
                    async move {
                        let json =
                            serde_json::to_string_pretty(&layout).map_err(|e| e.to_string())?;

                        let path = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .set_file_name(format!(
                                "{}.json",
                                layout.name.replace(" ", "_").to_lowercase()
                            ))
                            .save_file()
                            .await;

                        if let Some(path) = path {
                            std::fs::write(path.path(), json).map_err(|e| e.to_string())?;
                            Ok(())
                        } else {
                            Err("Export cancelled".to_string())
                        }
                    },
                    Message::LayoutExported,
                );
            }
            Message::ImportLayout => {
                return Task::perform(
                    async {
                        let path = rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            .await;

                        if let Some(path) = path {
                            let content =
                                std::fs::read_to_string(path.path()).map_err(|e| e.to_string())?;
                            let layout: config::SavedLayout =
                                serde_json::from_str(&content).map_err(|e| e.to_string())?;
                            Ok(layout)
                        } else {
                            Err("Import cancelled".to_string())
                        }
                    },
                    Message::LayoutImported,
                );
            }
            Message::LayoutExported(result) => match result {
                Ok(_) => self.push_toast("Layout exported successfully".to_string(), false),
                Err(e) => {
                    if e != "Export cancelled" {
                        self.push_toast(format!("Export failed: {}", e), true)
                    }
                }
            },
            Message::LayoutImported(result) => match result {
                Ok(layout) => {
                    let mut final_layout = layout;
                    final_layout.pane_layout = final_layout
                        .pane_layout
                        .take()
                        .and_then(config::prune_unsupported_pane_layout);
                    let base_name = final_layout.name.clone();
                    let mut counter = 1;
                    while self
                        .saved_layouts
                        .iter()
                        .any(|layout| layout.name == final_layout.name)
                    {
                        final_layout.name = format!("{} ({})", base_name, counter);
                        counter += 1;
                    }
                    self.saved_layouts.push(final_layout.clone());
                    self.push_toast(format!("Layout '{}' imported", final_layout.name), false);
                    self.persist_config();
                }
                Err(e) => {
                    if e != "Import cancelled" {
                        self.push_toast(format!("Import failed: {}", e), true)
                    }
                }
            },
            _ => {}
        }

        Task::none()
    }

    fn update_saved_layout_snapshot(&mut self, name: String) {
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

    fn reconcile_layout_rename_after_delete(&mut self, removed_index: Option<usize>) {
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

    fn rename_saved_layout(&mut self, index: usize) {
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
            self.active_layout_name = Some(new_name);
        }
        self.layout_rename_index = None;
        self.layout_rename_input.clear();
        self.persist_config();
    }
}

fn normalized_layout_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalized_layout_name;

    #[test]
    fn normalized_layout_name_trims_and_rejects_empty_names() {
        assert_eq!(
            normalized_layout_name("  Trading  "),
            Some("Trading".to_string())
        );
        assert_eq!(normalized_layout_name("   "), None);
    }
}
