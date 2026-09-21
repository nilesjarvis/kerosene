use super::*;
use crate::network_activity::ActivityKind;

fn entry(sequence: u64, provider: Provider, kind: ActivityKind) -> ActivityEntry {
    ActivityEntry {
        sequence,
        timestamp_ms: 0,
        provider,
        kind,
        operation: "allMids",
        method: "POST",
        request_id: None,
        elapsed_ms: None,
        bytes: None,
        proxied: false,
    }
}

#[test]
fn console_combines_provider_transport_and_clear_filters_in_reverse_order() {
    let mut state = ConsoleState::default();
    state.snapshot.entries.extend([
        entry(1, Provider::Hyperliquid, ActivityKind::HttpSend),
        entry(2, Provider::Hydromancer, ActivityKind::WsReceive),
        entry(3, Provider::Hyperliquid, ActivityKind::HttpResponse(429)),
        entry(4, Provider::Hyperliquid, ActivityKind::WsReceive),
    ]);
    assert_eq!(
        state
            .entries()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![4, 3, 2, 1]
    );
    state.provider = Provider::Hyperliquid;
    state.filter = ConsoleFilter::Errors;
    assert_eq!(
        state
            .entries()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![3]
    );
    state.filter = ConsoleFilter::Http;
    assert_eq!(state.entries().count(), 2);
    state.cleared_through = 3;
    assert_eq!(state.entries().count(), 0);
    state.filter = ConsoleFilter::WebSocket;
    assert_eq!(
        state
            .entries()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![4]
    );
}

#[test]
fn paused_console_keeps_its_history_snapshot() {
    let mut state = ConsoleState {
        paused: true,
        ..Default::default()
    };
    state
        .snapshot
        .entries
        .push_back(entry(123, Provider::Hyperliquid, ActivityKind::HttpSend));
    state.refresh();
    assert_eq!(
        state.entries().next().expect("retained entry").sequence,
        123
    );
}
