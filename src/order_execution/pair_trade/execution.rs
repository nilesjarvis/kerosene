use super::legs::PairLegOrder;
use crate::signing::{OrderKind, place_order};
use zeroize::Zeroizing;

// ---------------------------------------------------------------------------
// Pair Trade Order Sequence
// ---------------------------------------------------------------------------

pub(super) async fn execute_pair_order_sequence(
    key: Zeroizing<String>,
    leg_a: PairLegOrder,
    leg_b: PairLegOrder,
) -> Result<String, String> {
    let first = place_order(
        key.clone(),
        leg_a.asset,
        leg_a.is_buy,
        leg_a.price.clone(),
        leg_a.size.clone(),
        OrderKind::Market,
        false,
    )
    .await?;

    if first.is_error() {
        return Err(format!("Leg A failed: {}", first.summary()));
    }

    let second = place_order(
        key.clone(),
        leg_b.asset,
        leg_b.is_buy,
        leg_b.price.clone(),
        leg_b.size.clone(),
        OrderKind::Market,
        false,
    )
    .await;

    match second {
        Ok(resp_b) => {
            if resp_b.is_error() {
                let _ = place_order(
                    key,
                    leg_a.asset,
                    !leg_a.is_buy,
                    leg_a.price,
                    leg_a.size,
                    OrderKind::Market,
                    true,
                )
                .await;
                Err(format!(
                    "Leg B failed ({}). Leg A auto-revert attempted.",
                    resp_b.summary()
                ))
            } else {
                Ok(format!(
                    "Pair trade placed: {} then {}",
                    leg_a.coin, leg_b.coin
                ))
            }
        }
        Err(e) => {
            let _ = place_order(
                key,
                leg_a.asset,
                !leg_a.is_buy,
                leg_a.price,
                leg_a.size,
                OrderKind::Market,
                true,
            )
            .await;
            Err(format!(
                "Leg B request failed ({e}). Leg A auto-revert attempted."
            ))
        }
    }
}
