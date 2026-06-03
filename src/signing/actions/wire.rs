use serde::Serialize;

// ---------------------------------------------------------------------------
// Msgpack Wire Types
// ---------------------------------------------------------------------------

// Field order MUST match the Python SDK exactly, because msgpack preserves map
// key order and the action hash depends on the exact bytes.

/// Order wire: fields in Python SDK order: a, b, p, s, r, t
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct OrderTypeWire {
    pub(super) limit: LimitOrderWire,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LimitOrderWire {
    pub(super) tif: String,
}

/// Order action: fields in Python SDK order: type, orders, grouping
#[derive(Debug, Clone, Serialize)]
pub(in crate::signing) struct OrderAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) orders: Vec<OrderWire>,
    pub(super) grouping: String,
}

/// Cancel wire: fields in Python SDK order: a, o
#[derive(Debug, Clone, Serialize)]
pub(super) struct CancelWire {
    pub(super) a: u32,
    pub(super) o: u64,
}

/// Cancel action: fields in Python SDK order: type, cancels
#[derive(Debug, Clone, Serialize)]
pub(in crate::signing) struct CancelAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) cancels: Vec<CancelWire>,
}

/// Cancel-by-cloid wire: fields in docs order: asset, cloid
#[derive(Debug, Clone, Serialize)]
pub(super) struct CancelByCloidWire {
    pub(super) asset: u32,
    pub(super) cloid: String,
}

/// Cancel-by-cloid action: fields in docs order: type, cancels
#[derive(Debug, Clone, Serialize)]
pub(in crate::signing) struct CancelByCloidAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) cancels: Vec<CancelByCloidWire>,
}

/// Modify wire: fields in Python SDK order: oid, order
#[derive(Debug, Clone, Serialize)]
pub(super) struct ModifyWire {
    pub(super) oid: u64,
    pub(super) order: OrderWire,
}

/// Batch modify action: fields in Python SDK order: type, modifies
#[derive(Debug, Clone, Serialize)]
pub(in crate::signing) struct ModifyAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) modifies: Vec<ModifyWire>,
}

/// Update leverage action: fields in Python SDK/docs order: type, asset, isCross, leverage
#[derive(Debug, Clone, Serialize)]
pub(in crate::signing) struct UpdateLeverageAction {
    #[serde(rename = "type")]
    pub(super) action_type: String,
    pub(super) asset: u32,
    #[serde(rename = "isCross")]
    pub(super) is_cross: bool,
    pub(super) leverage: u32,
}
