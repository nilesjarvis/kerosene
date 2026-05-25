use super::*;

#[test]
fn sound_status_messages_are_deduplicated_until_drained() {
    report_sound_status("same audio warning", true);
    report_sound_status("same audio warning", true);

    let statuses = take_status_messages();

    assert_eq!(
        statuses,
        vec![SoundStatus {
            message: "same audio warning".to_string(),
            is_error: true,
        }]
    );
    assert!(take_status_messages().is_empty());
}
