use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;
use iced::Task;
use io::{export_layout_task, import_layout_task};
use names::normalized_layout_name;

mod io;
mod names;

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
                if !self.saved_layouts.iter().any(|layout| layout.name == name) {
                    self.active_layout_name = None;
                    self.push_toast(format!("Layout '{}' no longer exists", name), true);
                    return Task::none();
                }
                self.update_saved_layout_snapshot(name.clone());
                self.layout_rename_index = None;
                self.layout_rename_input.clear();
                self.push_toast(format!("Layout '{}' updated", name), false);
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
                self.remove_layout_hotkeys(&name);
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
                return export_layout_task(layout);
            }
            Message::ImportLayout => {
                return import_layout_task();
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
                    final_layout.widget_padding = final_layout.widget_padding.normalized();
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

#[cfg(test)]
mod tests;
