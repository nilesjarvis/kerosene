use crate::app_state::SensitiveString;

use iced::window;

// ---------------------------------------------------------------------------
// Add Account Window
// ---------------------------------------------------------------------------

/// Draft state for the dedicated add-account window. Nothing here touches the
/// account list or credential storage until the user submits; closing the
/// window drops the draft (the key input zeroizes on drop).
pub(crate) struct AddAccountWindowState {
    pub(crate) window_id: window::Id,
    pub(crate) name_input: String,
    pub(crate) address_input: String,
    pub(crate) key_input: SensitiveString,
    pub(crate) switch_on_add: bool,
    pub(crate) error: Option<String>,
}

impl AddAccountWindowState {
    pub(crate) fn new(window_id: window::Id) -> Self {
        Self {
            window_id,
            name_input: String::new(),
            address_input: String::new(),
            key_input: SensitiveString::default(),
            switch_on_add: true,
            error: None,
        }
    }
}
