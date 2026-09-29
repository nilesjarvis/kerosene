use super::*;

#[test]
fn timeline_response_carries_author_profile_image_urls() {
    let page = page_from_timeline_response(
        XFeedSource::Following,
        XTimelineResponse {
            data: Some(vec![XTweetPayload {
                id: "99".to_string(),
                text: "hello".to_string(),
                author_id: Some("42".to_string()),
                created_at: Some("2026-06-30T12:00:00.000Z".to_string()),
            }]),
            includes: Some(XTimelineIncludes {
                users: Some(vec![XUserPayload {
                    id: "42".to_string(),
                    username: "alice".to_string(),
                    name: "Alice".to_string(),
                    profile_image_url: Some("https://example.com/alice.jpg".to_string()),
                }]),
            }),
            meta: None,
        },
        1_000,
    );

    assert_eq!(
        page.posts[0].author_profile_image_url.as_deref(),
        Some("https://example.com/alice.jpg")
    );
    assert_eq!(page.posts[0].author_profile_key(), "id:42");
}
