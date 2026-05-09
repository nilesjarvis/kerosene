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
            Message::LoadLayout(layout) => {
                self.active_layout_name = Some(layout.name.clone());
                let task = self.apply_layout(layout);
                self.persist_config();
                return task;
            }
            Message::DeleteLayout(name) => {
                self.saved_layouts.retain(|layout| layout.name != name);
                if self.active_layout_name.as_deref() == Some(name.as_str()) {
                    self.active_layout_name = None;
                }
                self.persist_config();
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
}
