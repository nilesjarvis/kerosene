use crate::config;
use crate::message::Message;

use iced::Task;

// ---------------------------------------------------------------------------
// Layout Import/Export Tasks
// ---------------------------------------------------------------------------

pub(super) fn export_layout_task(layout: config::SavedLayout) -> Task<Message> {
    Task::perform(
        async move {
            let json = serde_json::to_string_pretty(&layout).map_err(|e| e.to_string())?;

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
        |result| Message::LayoutExported(result.into()),
    )
}

pub(super) fn import_layout_task() -> Task<Message> {
    Task::perform(
        async {
            let path = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
                .await;

            if let Some(path) = path {
                let content = std::fs::read_to_string(path.path()).map_err(|e| e.to_string())?;
                let layout: config::SavedLayout =
                    serde_json::from_str(&content).map_err(|e| e.to_string())?;
                Ok(layout)
            } else {
                Err("Import cancelled".to_string())
            }
        },
        |result| Message::LayoutImported(result.into()),
    )
}
