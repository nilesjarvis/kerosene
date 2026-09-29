use super::*;
use crate::config::KeroseneConfig;
use crate::wallet_state::AddressBookEntry;

const ADDRESS: &str = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";

#[test]
fn wallet_display_preserves_normalization_and_first_nonblank_label_precedence() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    for (local, remote, expected) in [
        (None, None, None),
        (Some(" Local "), None, Some("Local")),
        (None, Some(" Remote "), Some("Remote")),
        (Some("Local"), Some(" Remote "), Some("Remote")),
        (Some(" Local "), Some(""), Some("Local")),
        (Some(" Local "), Some("\t\u{2003}\n"), Some("Local")),
        (Some(" \n"), Some("\t"), None),
        (None, Some(""), None),
    ] {
        terminal.address_book.clear();
        terminal.wallet_tracker.remote_database.entries.clear();
        for (book, label) in [
            (&mut terminal.address_book, local),
            (&mut terminal.wallet_tracker.remote_database.entries, remote),
        ] {
            if let Some(label) = label {
                book.insert(
                    ADDRESS.to_string(),
                    AddressBookEntry {
                        label: label.to_string(),
                        color: Some("#ff00ff".into()),
                        tags: vec!["metadata-without-label".into()],
                    },
                );
            }
        }
        for input in [
            ADDRESS.to_string(),
            ADDRESS.to_uppercase(),
            format!(" \u{2003}{}\n", ADDRESS.to_uppercase()),
        ] {
            assert_eq!(terminal.wallet_label(&input), expected);
            assert_eq!(terminal.wallet_is_remote(&input), remote.is_some());
            let display = terminal.wallet_display(&input);
            assert_eq!(display.has_label, expected.is_some());
            assert_eq!(display.primary, expected.unwrap_or("0xabcd...abcd"));
            assert_eq!(
                display.secondary,
                if expected.is_some() {
                    "0xabcd...abcd"
                } else {
                    ADDRESS
                }
            );
        }
    }
}

#[test]
fn invalid_wallet_display_preserves_raw_text_and_unicode_shortening_without_labels() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    for (input, short) in [
        ("", ""),
        ("  bad  ", "  bad  "),
        ("0XNOTHEX", "0XNOTHEX"),
        ("abcdef1234", "abcdef1234"),
        ("abcdef12345", "abcdef...2345"),
        ("αβγδεζηθικλ", "αβγδεζ...θικλ"),
        ("😀😁😂😃😄😅😆😉😊🙂", "😀😁😂😃😄😅😆😉😊🙂"),
        (
            "0xgggggggggggggggggggggggggggggggggggggggg",
            "0xgggg...gggg",
        ),
    ] {
        for book in [
            &mut terminal.address_book,
            &mut terminal.wallet_tracker.remote_database.entries,
        ] {
            book.clear();
            for key in [input.to_string(), input.trim().to_lowercase()] {
                book.insert(
                    key,
                    AddressBookEntry {
                        label: "Must not label an invalid address".into(),
                        ..Default::default()
                    },
                );
            }
        }
        assert!(terminal.wallet_label(input).is_none());
        assert!(!terminal.wallet_is_remote(input));
        let display = terminal.wallet_display(input);
        assert!(!display.has_label);
        assert_eq!(display.primary, short);
        assert_eq!(display.secondary, input);
    }
}
