mod asset_context;
mod books;
mod candles;

use crate::account::AssetContext;
use crate::api::Candle;

pub use asset_context::ws_asset_ctx_stream_keyed;
pub use books::ws_book_stream_keyed;
pub use candles::{ws_candle_stream_keyed, ws_spaghetti_candle_stream};

type KeyedAssetContext = (u64, String, AssetContext);
type KeyedCandleUpdate = (u64, String, String, Candle);
