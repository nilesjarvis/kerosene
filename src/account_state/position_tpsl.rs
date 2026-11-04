use crate::canvas_state::WorkspaceId;
use crate::signing::{PositionTpslOrder, TpslKind};

#[derive(Default)]
pub(crate) struct PositionTpslState {
    pub(crate) form: Option<PositionTpslForm>,
    pub(crate) pending: Option<PositionTpslSubmission>,
    pub(crate) next_id: u64,
}

impl PositionTpslState {
    pub(crate) fn clear(&mut self) {
        self.form = None;
        self.pending = None;
        // Keep the generation across account switches to reject late results.
    }
}

pub(crate) struct PositionTpslForm {
    pub(crate) workspace: WorkspaceId,
    pub(crate) visible: bool,
    pub(crate) account_address: String,
    pub(crate) coin: String,
    pub(crate) is_long: bool,
    pub(crate) take_profit: String,
    pub(crate) stop_loss: String,
    pub(crate) status: Option<(String, bool)>,
}

pub(crate) struct PositionTpslSubmission {
    pub(crate) id: u64,
    pub(crate) account_address: String,
    pub(crate) orders: Vec<PositionTpslOrder>,
    pub(crate) outcomes: Vec<TpslOutcome>,
    pub(crate) in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TpslOutcome {
    Accepted,
    Rejected(String),
    Unknown,
}

impl PositionTpslForm {
    pub(crate) fn input_mut(&mut self, kind: TpslKind) -> &mut String {
        match kind {
            TpslKind::TakeProfit => &mut self.take_profit,
            TpslKind::StopLoss => &mut self.stop_loss,
        }
    }
}
