use super::super::{HydromancerCommand, HydromancerReconnectGate};
use super::subscriptions::ActiveHydromancerSubscriptions;

use std::time::Duration;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Pre-connect Lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HydromancerTaskControlFlow {
    Continue,
    Shutdown,
}

pub(super) fn handle_preconnect_hydromancer_command(
    cmd: HydromancerCommand,
    active_subs: &mut ActiveHydromancerSubscriptions,
) -> HydromancerTaskControlFlow {
    match cmd {
        HydromancerCommand::Subscribe { topic, payload } => {
            active_subs.subscribe(topic, payload);
            HydromancerTaskControlFlow::Continue
        }
        HydromancerCommand::Unsubscribe { topic, payload } => {
            active_subs.unsubscribe(topic, payload);
            HydromancerTaskControlFlow::Continue
        }
        HydromancerCommand::Reconnect => HydromancerTaskControlFlow::Continue,
        HydromancerCommand::Shutdown => HydromancerTaskControlFlow::Shutdown,
    }
}

pub(super) fn drain_pending_hydromancer_shutdown(
    cmd_rx: &mut mpsc::UnboundedReceiver<HydromancerCommand>,
    active_subs: &mut ActiveHydromancerSubscriptions,
    reconnect_gate: &HydromancerReconnectGate,
) -> HydromancerTaskControlFlow {
    while let Ok(cmd) = cmd_rx.try_recv() {
        reconnect_gate.note_dequeued(&cmd);
        if handle_preconnect_hydromancer_command(cmd, active_subs)
            == HydromancerTaskControlFlow::Shutdown
        {
            return HydromancerTaskControlFlow::Shutdown;
        }
    }
    HydromancerTaskControlFlow::Continue
}

pub(super) async fn hydromancer_wait_for_subscription_or_shutdown(
    cmd_rx: &mut mpsc::UnboundedReceiver<HydromancerCommand>,
    active_subs: &mut ActiveHydromancerSubscriptions,
    idle_timeout: Duration,
    reconnect_gate: &HydromancerReconnectGate,
) -> HydromancerTaskControlFlow {
    let mut idle_fut = Box::pin(tokio::time::sleep(idle_timeout));

    loop {
        if !active_subs.is_empty() {
            return HydromancerTaskControlFlow::Continue;
        }

        let cmd_fut = Box::pin(cmd_rx.recv());
        match futures::future::select(cmd_fut, idle_fut).await {
            futures::future::Either::Left((cmd, remaining_idle)) => {
                idle_fut = remaining_idle;
                match cmd {
                    Some(cmd) => {
                        reconnect_gate.note_dequeued(&cmd);
                        if handle_preconnect_hydromancer_command(cmd, active_subs)
                            == HydromancerTaskControlFlow::Shutdown
                        {
                            return HydromancerTaskControlFlow::Shutdown;
                        }
                    }
                    None => return HydromancerTaskControlFlow::Shutdown,
                }
            }
            futures::future::Either::Right((_, _)) => {
                return HydromancerTaskControlFlow::Shutdown;
            }
        }
    }
}

pub(super) async fn hydromancer_sleep_or_shutdown(
    cmd_rx: &mut mpsc::UnboundedReceiver<HydromancerCommand>,
    active_subs: &mut ActiveHydromancerSubscriptions,
    duration: Duration,
    reconnect_gate: &HydromancerReconnectGate,
) -> HydromancerTaskControlFlow {
    if active_subs.is_empty() {
        return HydromancerTaskControlFlow::Continue;
    }

    let mut sleep_fut = Box::pin(tokio::time::sleep(duration));

    loop {
        let cmd_fut = Box::pin(cmd_rx.recv());
        match futures::future::select(cmd_fut, sleep_fut).await {
            futures::future::Either::Left((cmd, remaining_sleep)) => {
                sleep_fut = remaining_sleep;
                match cmd {
                    Some(cmd) => {
                        reconnect_gate.note_dequeued(&cmd);
                        if handle_preconnect_hydromancer_command(cmd, active_subs)
                            == HydromancerTaskControlFlow::Shutdown
                        {
                            return HydromancerTaskControlFlow::Shutdown;
                        }
                        if active_subs.is_empty() {
                            return HydromancerTaskControlFlow::Continue;
                        }
                    }
                    None => return HydromancerTaskControlFlow::Shutdown,
                }
            }
            futures::future::Either::Right((_, _)) => {
                return HydromancerTaskControlFlow::Continue;
            }
        }
    }
}
