use serde::Serialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Msgpack Wire Types
// ---------------------------------------------------------------------------

// Field order MUST match the Python SDK exactly, because msgpack preserves map
// key order and the action hash depends on the exact bytes.

/// Order wire: fields in Python SDK order: a, b, p, s, r, t
#[derive(Clone, Serialize)]
pub(super) struct OrderWire {
    pub(super) a: u32,
    pub(super) b: bool,
    pub(super) p: String,
    pub(super) s: String,
    pub(super) r: bool,
    pub(super) t: OrderTypeWire,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) c: Option<String>,
}

impl fmt::Debug for OrderWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderWire")
            .field("a", &self.a)
            .field("b", &self.b)
            .field("p", &"<redacted>")
            .field("s", &"<redacted>")
            .field("r", &self.r)
            .field("t", &self.t)
            .field("has_cloid", &self.c.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OrderTypeWire {
    pub(super) limit: LimitOrderWire,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LimitOrderWire {
    pub(super) tif: String,
}

/// Order action: fields in Python SDK order: type, orders, grouping
#[derive(Clone, Serialize)]
pub(in crate::signing) struct OrderAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) orders: Vec<OrderWire>,
    pub(super) grouping: String,
}

impl fmt::Debug for OrderAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderAction")
            .field("action_type", &self.action_type)
            .field("orders_count", &self.orders.len())
            .field("grouping", &self.grouping)
            .finish()
    }
}

/// Cancel wire: fields in Python SDK order: a, o
#[derive(Clone, Serialize)]
pub(super) struct CancelWire {
    pub(super) a: u32,
    pub(super) o: u64,
}

impl fmt::Debug for CancelWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelWire")
            .field("a", &self.a)
            .field("o", &"<redacted>")
            .finish()
    }
}

/// Cancel action: fields in Python SDK order: type, cancels
#[derive(Clone, Serialize)]
pub(in crate::signing) struct CancelAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) cancels: Vec<CancelWire>,
}

impl fmt::Debug for CancelAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelAction")
            .field("action_type", &self.action_type)
            .field("cancels_count", &self.cancels.len())
            .finish()
    }
}

/// Cancel-by-cloid wire: fields in docs order: asset, cloid
#[derive(Clone, Serialize)]
pub(super) struct CancelByCloidWire {
    pub(super) asset: u32,
    pub(super) cloid: String,
}

impl fmt::Debug for CancelByCloidWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelByCloidWire")
            .field("asset", &self.asset)
            .field("cloid", &"<redacted>")
            .finish()
    }
}

/// Cancel-by-cloid action: fields in docs order: type, cancels
#[derive(Clone, Serialize)]
pub(in crate::signing) struct CancelByCloidAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) cancels: Vec<CancelByCloidWire>,
}

impl fmt::Debug for CancelByCloidAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelByCloidAction")
            .field("action_type", &self.action_type)
            .field("cancels_count", &self.cancels.len())
            .finish()
    }
}

/// Modify wire: fields in Python SDK order: oid, order
#[derive(Clone, Serialize)]
pub(super) struct ModifyWire {
    pub(super) oid: u64,
    pub(super) order: OrderWire,
}

impl fmt::Debug for ModifyWire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyWire")
            .field("oid", &"<redacted>")
            .field("order", &self.order)
            .finish()
    }
}

/// Batch modify action: fields in Python SDK order: type, modifies
#[derive(Clone, Serialize)]
pub(in crate::signing) struct ModifyAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) modifies: Vec<ModifyWire>,
}

impl fmt::Debug for ModifyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModifyAction")
            .field("action_type", &self.action_type)
            .field("modifies_count", &self.modifies.len())
            .finish()
    }
}

/// Update leverage action: fields in Python SDK/docs order: type, asset, isCross, leverage
#[derive(Clone, Serialize)]
pub(in crate::signing) struct UpdateLeverageAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) asset: u32,
    #[serde(rename = "isCross")]
    pub(super) is_cross: bool,
    pub(super) leverage: u32,
}

impl fmt::Debug for UpdateLeverageAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateLeverageAction")
            .field("action_type", &self.action_type)
            .field("asset", &"<redacted>")
            .field("is_cross", &self.is_cross)
            .field("leverage", &"<redacted>")
            .finish()
    }
}
