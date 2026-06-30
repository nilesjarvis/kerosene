use crate::x_feed::XFeedSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct XFeedConfig {
    pub id: u64,
    #[serde(default)]
    pub source: XFeedSource,
}
