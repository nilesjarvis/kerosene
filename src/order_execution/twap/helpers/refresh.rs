use crate::signing::ExchangeResponse;

// ---------------------------------------------------------------------------
// TWAP Refresh Policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::order_execution::twap) enum TwapAccountRefresh {
    None,
    OnTerminal,
    Immediate,
}

impl TwapAccountRefresh {
    pub(in crate::order_execution::twap) fn should_refresh(self, twap_is_terminal: bool) -> bool {
        match self {
            Self::None => false,
            Self::OnTerminal => twap_is_terminal,
            Self::Immediate => true,
        }
    }
}

pub(in crate::order_execution::twap) fn twap_place_result_refresh_policy(
    result: &Result<ExchangeResponse, String>,
) -> TwapAccountRefresh {
    match result {
        Err(_) => TwapAccountRefresh::Immediate,
        Ok(response) if response.is_ambiguous_order_result() => TwapAccountRefresh::Immediate,
        Ok(response) if response.is_error() => TwapAccountRefresh::None,
        Ok(_) => TwapAccountRefresh::OnTerminal,
    }
}
