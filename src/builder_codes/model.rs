use std::fmt;

use serde::{Deserialize, Serialize};

/// One global rate must fit the stricter perpetual-market maximum (10 bps).
const MAX_FEE_TENTHS_BPS: u32 = 100;

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BuilderCodeConfig {
    pub(crate) enabled: bool,
    pub(crate) address: String,
    pub(crate) fee_tenths_bps: u32,
}

impl BuilderCodeConfig {
    pub(crate) fn resolve(&self) -> Result<Option<BuilderFee>, String> {
        if !self.enabled {
            return Ok(None);
        }
        BuilderFee::new(&self.address, self.fee_tenths_bps).map(Some)
    }
}

impl fmt::Debug for BuilderCodeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BuilderCodeConfig")
            .field("enabled", &self.enabled)
            .field("address", &"<redacted>")
            .field("fee_tenths_bps", &self.fee_tenths_bps)
            .finish()
    }
}

/// Validated routing captured with an order's account, never a live settings reference.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BuilderFee {
    address: String,
    fee_tenths_bps: u32,
}

impl BuilderFee {
    pub(crate) fn new(address: &str, fee_tenths_bps: u32) -> Result<Self, String> {
        let address = normalize_builder_address(address)?;
        if !(1..=MAX_FEE_TENTHS_BPS).contains(&fee_tenths_bps) {
            return Err("Builder fee must be between 0.1 and 10 basis points".to_string());
        }
        Ok(Self {
            address,
            fee_tenths_bps,
        })
    }

    pub(crate) fn from_inputs(address: &str, fee_bps: &str) -> Result<Self, String> {
        let fee_bps = fee_bps.trim();
        let (whole, fractional) = match fee_bps.split_once('.') {
            Some((whole, fractional)) if fractional.len() == 1 => (whole, Some(fractional)),
            Some(_) => return Err(invalid_fee_precision()),
            None => (fee_bps, None),
        };
        if whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || fractional.is_some_and(|value| !value.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(invalid_fee_precision());
        }
        let whole: u32 = whole.parse().map_err(|_| invalid_fee_precision())?;
        let fractional = fractional
            .map(|value| u32::from(value.as_bytes()[0] - b'0'))
            .unwrap_or_default();
        let fee_tenths_bps = whole
            .checked_mul(10)
            .and_then(|value| value.checked_add(fractional))
            .ok_or_else(invalid_fee_precision)?;
        Self::new(address, fee_tenths_bps)
    }

    pub(crate) fn address(&self) -> &str {
        &self.address
    }

    pub(crate) fn fee_tenths_bps(&self) -> u32 {
        self.fee_tenths_bps
    }

    pub(crate) fn fee_bps_text(&self) -> String {
        if self.fee_tenths_bps.is_multiple_of(10) {
            (self.fee_tenths_bps / 10).to_string()
        } else {
            format!("{}.{}", self.fee_tenths_bps / 10, self.fee_tenths_bps % 10)
        }
    }

    /// Approval rates use percent strings: one tenth of a basis point is 0.001%.
    pub(crate) fn max_fee_rate(&self) -> String {
        format!("0.{:03}%", self.fee_tenths_bps)
    }
}

impl fmt::Debug for BuilderFee {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BuilderFee")
            .field("address", &"<redacted>")
            .field("fee_tenths_bps", &self.fee_tenths_bps)
            .finish()
    }
}

pub(crate) fn normalize_builder_address(address: &str) -> Result<String, String> {
    let address = address.trim();
    let valid = address.strip_prefix("0x").is_some_and(|hex| {
        hex.len() == 40
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            && hex.bytes().any(|byte| byte != b'0')
    });
    if !valid {
        return Err("Builder address must be a nonzero 0x-prefixed 20-byte address".to_string());
    }
    Ok(address.to_ascii_lowercase())
}

fn invalid_fee_precision() -> String {
    "Enter a builder fee in basis points, using at most one decimal place".to_string()
}
