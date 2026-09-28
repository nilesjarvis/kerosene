use super::*;
use crate::wallet_cluster_state::WalletClusterMember;

#[test]
fn prepared_cluster_leg_keeps_request_fields_context_and_captured_member_target() {
    use crate::order_execution::PreparedExchangeOrder;
    use crate::signing::CapturedAgentKey;
    use zeroize::Zeroizing;

    for (surface, market_type, kind) in [
        (
            OrderSurface::Cluster,
            MarketType::Perp,
            ExchangeOrderKind::Limit,
        ),
        (
            OrderSurface::ClusterClose,
            MarketType::Perp,
            ExchangeOrderKind::Market,
        ),
        (
            OrderSurface::Cluster,
            MarketType::Spot,
            ExchangeOrderKind::LimitIoc,
        ),
        (
            OrderSurface::Cluster,
            MarketType::Outcome,
            ExchangeOrderKind::Limit,
        ),
    ] {
        let member = ClusterTradingMember {
            profile_secret_id: "member".into(),
            label: "Member".into(),
            address: ADDRESS.into(),
            agent_key: CapturedAgentKey::for_account(
                Zeroizing::new("synthetic-invalid-key".into()),
                Some(ADDRESS),
            )
            .expect("captured subaccount"),
            weight: 2.5,
        };
        let order = PreparedExchangeOrder {
            surface,
            symbol_key: "BTC".into(),
            asset: 7,
            is_buy: false,
            price: "123.45".into(),
            size: "0.25".into(),
            order_kind: kind,
            reduce_only: true,
            market_type,
        };
        // The recorded direction and prepared request direction remain separate inputs.
        let leg = PreparedClusterLeg::new(member, order, true);
        assert_eq!(leg.member.profile_secret_id, "member");
        assert_eq!(leg.member.label, "Member");
        assert_eq!(leg.member.address, ADDRESS);
        assert_eq!(leg.member.weight, 2.5);
        assert_eq!(leg.member.agent_key.vault_address(), Some(ADDRESS));
        assert!(!leg.member.agent_key.is_empty());
        assert!(leg.is_buy);
        assert_eq!(leg.market_type, market_type);
        assert_eq!(leg.price, "123.45");
        assert_eq!(leg.size, "0.25");
        assert_eq!(leg.request.asset, 7);
        assert!(!leg.request.is_buy);
        assert_eq!(leg.request.price, "123.45");
        assert_eq!(leg.request.size, "0.25");
        assert_eq!(leg.request.order_kind, kind);
        assert!(leg.request.reduce_only);
        assert_eq!(
            leg.request.cloid.as_deref(),
            Some(leg.context.cloid.as_str())
        );
        assert!(!leg.context.cloid.is_empty());
        assert_eq!(leg.context.account_address, ADDRESS);
        assert_eq!(leg.context.surface, surface);
        assert_eq!(leg.context.symbol_key, "BTC");
        assert_eq!(leg.context.order_kind, kind);
    }
}

const ADDRESS: &str = "0x1111111111111111111111111111111111111111";

#[test]
fn cluster_trading_members_excludes_zero_weight_members_when_required() {
    use crate::config::AccountProfile;
    use crate::wallet_cluster_state::{WalletCluster, WalletClusterMember};

    let terminal = {
        let mut terminal = TradingTerminal::boot().0;
        terminal.accounts = vec![
            AccountProfile {
                master_address: None,
                secret_id: "disabled-profile".to_string(),
                name: "Disabled".to_string(),
                wallet_address: ADDRESS.to_string(),
                agent_key: "disabled-agent-key".to_string().into(),
                hydromancer_api_key: String::new().into(),
            },
            AccountProfile {
                master_address: None,
                secret_id: "enabled-profile".to_string(),
                name: "Enabled".to_string(),
                wallet_address: "0x2222222222222222222222222222222222222222".to_string(),
                agent_key: "enabled-agent-key".to_string().into(),
                hydromancer_api_key: String::new().into(),
            },
        ];
        terminal
    };
    let cluster = WalletCluster {
        id: "cluster".to_string(),
        name: "Cluster".to_string(),
        members: vec![
            WalletClusterMember {
                profile_secret_id: "disabled-profile".to_string(),
                weight: 0.0,
                weight_input: "0".to_string(),
            },
            WalletClusterMember {
                profile_secret_id: "enabled-profile".to_string(),
                weight: 1.0,
                weight_input: "1".to_string(),
            },
        ],
    };

    let members = terminal
        .cluster_trading_members(&cluster, true)
        .expect("positive-weight member should be eligible");

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].profile_secret_id, "enabled-profile");
}

#[test]
fn cluster_subaccounts_capture_distinct_targets_with_the_same_parent_key() {
    let mut terminal = TradingTerminal::boot().0;
    let child_a = "0x2222222222222222222222222222222222222222";
    let child_b = "0x3333333333333333333333333333333333333333";
    terminal.accounts = [child_a, child_b]
        .into_iter()
        .enumerate()
        .map(|(index, address)| crate::config::AccountProfile {
            secret_id: format!("child-{index}"),
            name: format!("Child {index}"),
            wallet_address: address.to_string(),
            master_address: Some(ADDRESS.to_string()),
            agent_key: "parent-agent-key".to_string().into(),
            hydromancer_api_key: String::new().into(),
        })
        .collect();
    let cluster = WalletCluster {
        id: "children".to_string(),
        name: "Children".to_string(),
        members: terminal
            .accounts
            .iter()
            .map(|profile| WalletClusterMember {
                profile_secret_id: profile.secret_id.clone(),
                weight: 1.0,
                weight_input: "1".to_string(),
            })
            .collect(),
    };
    let members = terminal
        .cluster_trading_members(&cluster, true)
        .expect("child members");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].address, child_a);
    assert_eq!(members[0].agent_key.vault_address(), Some(child_a));
    assert_eq!(members[1].address, child_b);
    assert_eq!(members[1].agent_key.vault_address(), Some(child_b));
    assert_eq!(members[0].agent_key.as_str(), members[1].agent_key.as_str());

    terminal.accounts[1].master_address = Some(child_b.to_string());
    assert!(terminal.cluster_trading_members(&cluster, true).is_err());
}
