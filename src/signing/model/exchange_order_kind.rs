use super::order_kind::OrderKind;

/// Exchange order type accepted by the signing layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeOrderKind {
    Market,
    Limit,
    LimitIoc,
}

impl ExchangeOrderKind {
    pub(crate) fn tif(self) -> &'static str {
        match self {
            Self::Market | Self::LimitIoc => "Ioc",
            Self::Limit => "Gtc",
        }
    }
}

impl TryFrom<OrderKind> for ExchangeOrderKind {
    type Error = &'static str;

    fn try_from(value: OrderKind) -> Result<Self, Self::Error> {
        match value {
            OrderKind::Market => Ok(Self::Market),
            OrderKind::Limit => Ok(Self::Limit),
            OrderKind::LimitIoc => Ok(Self::LimitIoc),
            OrderKind::Chase | OrderKind::Twap => {
                Err("Advanced order modes are not exchange order types")
            }
        }
    }
}
