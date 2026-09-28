use super::POSITION_EPSILON;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;
use crate::wallet_cluster_state::{
    WalletClusterCloseSide, WalletClusterPositionMember, WalletClusterPositionSummary,
};

impl TradingTerminal {
    pub(super) fn cluster_leg_reduces_position(
        &self,
        profile_secret_id: &str,
        symbol: &str,
        is_buy: bool,
        size: &str,
    ) -> bool {
        let Some(size) = parse_finite_number(size).filter(|size| *size > POSITION_EPSILON) else {
            return false;
        };
        let Some(state) = self.wallet_clusters.member_data.get(profile_secret_id) else {
            return false;
        };
        let Some(data) = state.data.as_ref() else {
            return false;
        };
        let available: f64 = data
            .positions
            .iter()
            .filter(|position| position.asset_position.position.coin == symbol)
            .filter_map(|position| parse_finite_number(&position.asset_position.position.szi))
            .filter(|szi| {
                if is_buy {
                    *szi < -POSITION_EPSILON
                } else {
                    *szi > POSITION_EPSILON
                }
            })
            .map(f64::abs)
            .sum();
        available + POSITION_EPSILON >= size
    }

    pub(crate) fn wallet_cluster_position_summaries(&self) -> Vec<WalletClusterPositionSummary> {
        let mut summaries: Vec<WalletClusterPositionSummary> = Vec::new();
        let Some(cluster) = self.wallet_clusters.selected_cluster() else {
            return summaries;
        };

        for member in &cluster.members {
            let Some(state) = self
                .wallet_clusters
                .member_data
                .get(&member.profile_secret_id)
            else {
                continue;
            };
            let Some(data) = state.data.as_ref() else {
                continue;
            };
            let label = self
                .accounts
                .iter()
                .find(|profile| profile.secret_id == member.profile_secret_id)
                .map(|profile| profile.name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| self.wallet_display(&state.address).primary);
            for position in &data.positions {
                let coin = position.asset_position.position.coin.clone();
                let Some(size) = parse_finite_number(&position.asset_position.position.szi) else {
                    continue;
                };
                if size.abs() <= POSITION_EPSILON {
                    continue;
                }
                let entry_price = parse_finite_number(&position.asset_position.position.entry_px);
                let value = parse_finite_number(&position.asset_position.position.position_value)
                    .or_else(|| {
                        self.resolve_mid_for_symbol(&coin)
                            .map(|mid| mid * size.abs())
                    });
                let unrealized_pnl =
                    parse_finite_number(&position.asset_position.position.unrealized_pnl);
                let summary_index = summaries.iter().position(|summary| summary.symbol == coin);
                let index = if let Some(index) = summary_index {
                    index
                } else {
                    summaries.push(WalletClusterPositionSummary {
                        symbol: coin.clone(),
                        net_size: 0.0,
                        long_size: 0.0,
                        short_size: 0.0,
                        value: Some(0.0),
                        unrealized_pnl: Some(0.0),
                        members: Vec::new(),
                    });
                    summaries.len() - 1
                };
                let summary = &mut summaries[index];
                summary.net_size += size;
                if size > 0.0 {
                    summary.long_size += size;
                } else {
                    summary.short_size += size.abs();
                }
                add_optional(&mut summary.value, value);
                add_optional(&mut summary.unrealized_pnl, unrealized_pnl);
                summary.members.push(WalletClusterPositionMember {
                    profile_secret_id: member.profile_secret_id.clone(),
                    address: state.address.clone(),
                    label: label.clone(),
                    dex: position.dex.clone(),
                    size,
                    entry_price,
                    value,
                    unrealized_pnl,
                });
            }
        }

        summaries.sort_by(|a, b| {
            b.value
                .unwrap_or(0.0)
                .partial_cmp(&a.value.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        summaries
    }
}

fn add_optional(target: &mut Option<f64>, value: Option<f64>) {
    match (target.as_mut(), value) {
        (Some(total), Some(value)) => *total += value,
        (None, Some(value)) => *target = Some(value),
        (Some(_), None) => *target = None,
        (None, None) => {}
    }
}

/// Total size to close for one member on `symbol`, summed across every dex it
/// holds a same-side position on, scaled by `fraction`. Returns `None` when the
/// member has no closeable position on that side (or the result rounds to ~0).
///
/// Summing across dexes is safe because cluster closes are reduce-only
/// (`Fixed(true)`): the exchange caps each leg's fill at the on-venue position,
/// so an over-estimate can only under-close, never over-close or flip.
pub(super) fn cluster_close_size_for_member(
    summaries: &[WalletClusterPositionSummary],
    symbol: &str,
    profile_secret_id: &str,
    side: WalletClusterCloseSide,
    fraction: f64,
) -> Option<f64> {
    summaries
        .iter()
        .find(|summary| summary.symbol == symbol)
        .map(|summary| {
            summary
                .members
                .iter()
                .filter(|position| {
                    position.profile_secret_id == profile_secret_id
                        && ((matches!(side, WalletClusterCloseSide::Long) && position.size > 0.0)
                            || (matches!(side, WalletClusterCloseSide::Short)
                                && position.size < 0.0))
                })
                .map(|position| position.size.abs())
                .sum::<f64>()
        })
        .map(|total| total * fraction)
        .filter(|size| *size > POSITION_EPSILON)
}

#[cfg(test)]
mod tests;
