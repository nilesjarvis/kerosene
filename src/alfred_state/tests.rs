use super::{AlfredCommandId, AlfredCommandKind};
use crate::app_state::TradingTerminal;

#[test]
fn alfred_defaults_to_add_widget_commands() {
    let terminal = TradingTerminal::boot().0;
    let commands = terminal.alfred_filtered_commands();

    assert!(
        commands
            .iter()
            .any(|command| command.id == AlfredCommandId::AddCandlestickChart)
    );
    assert!(
        commands
            .iter()
            .all(|command| command.kind != AlfredCommandKind::Trading)
    );
}

#[test]
fn alfred_shows_only_trade_draft_for_trade_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "buy btc".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
}

#[test]
fn alfred_shows_only_trade_draft_for_chase_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "chase 1k HYPE".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NaturalLanguageTrading);
}

#[test]
fn alfred_shows_only_nuke_command_for_nuke_query() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "nuke".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
}

#[test]
fn alfred_treats_close_all_as_nuke_command() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "close all".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::NukePositions);
}

#[test]
fn alfred_shows_only_close_position_command_for_close_queries() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.alfred.query = "close HYPE".to_string();

    let commands = terminal.alfred_filtered_commands();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id, AlfredCommandId::ClosePosition);
}
