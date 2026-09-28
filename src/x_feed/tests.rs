use super::*;

#[test]
fn x_feed_state_debug_redacts_tokens_and_status() {
    let mut state = XFeedState::new(&[], "", "", "");
    state.access_token_input = sensitive_string("token-input");
    state.oauth_client_id_input = sensitive_string("client-input");
    state.refresh_token_input = sensitive_string("refresh-input");
    state.access_token = sensitive_string("saved-token");
    state.oauth_client_id = sensitive_string("saved-client");
    state.refresh_token = sensitive_string("saved-refresh");
    state.status = Some(("auth_token=token-input failed".to_string(), true));

    let rendered = format!("{state:?}");

    assert!(!rendered.contains("token-input"));
    assert!(!rendered.contains("client-input"));
    assert!(!rendered.contains("refresh-input"));
    assert!(!rendered.contains("saved-token"));
    assert!(!rendered.contains("saved-client"));
    assert!(!rendered.contains("saved-refresh"));
    assert!(rendered.contains("<redacted>"));
}

#[test]
fn direct_access_token_commit_clears_refresh_credentials() {
    let mut state = XFeedState::new(&[], "old-access", "old-client", "old-refresh");
    state.token_refreshing = true;
    let previous_refresh_request_id = state.token_refresh_request_id;

    assert!(state.commit_access_token("new-access"));

    let (access_token, client_id, refresh_token) = state.oauth_credentials_for_secret();
    assert_eq!(access_token.as_str(), "new-access");
    assert_eq!(client_id.as_str(), "");
    assert_eq!(refresh_token.as_str(), "");
    assert!(!state.has_refresh_credentials());
    assert!(!state.token_refreshing);
    assert!(state.token_refresh_request_id > previous_refresh_request_id);
}

#[test]
fn clear_access_token_invalidates_pending_refresh_request() {
    let mut state = XFeedState::new(&[], "", "", "");
    state.pending_oauth_client_id = sensitive_string("pending-client");
    state.pending_refresh_token = sensitive_string("pending-refresh");
    state.token_refreshing = true;
    let previous_refresh_request_id = state.token_refresh_request_id;

    state.clear_access_token();

    let (access_token, client_id, refresh_token) = state.oauth_credentials_for_secret();
    assert_eq!(access_token.as_str(), "");
    assert_eq!(client_id.as_str(), "");
    assert_eq!(refresh_token.as_str(), "");
    assert!(state.pending_oauth_credentials_for_secret().is_none());
    assert!(!state.token_refreshing);
    assert!(state.token_refresh_request_id > previous_refresh_request_id);
}

#[test]
fn credential_candidates_preserve_pending_values_until_valid_and_return_owned_snapshots() {
    let mut state = XFeedState::new(&[], " saved-access ", " saved-client ", " saved-refresh ");
    state.pending_access_token = sensitive_string(" pending-access ");
    state.pending_oauth_client_id = sensitive_string(" pending-client ");
    state.pending_refresh_token = sensitive_string(" pending-refresh ");
    state.access_token_input = sensitive_string(" \t ");
    state.oauth_client_id_input = sensitive_string(" new-client ");
    state.refresh_token_input = sensitive_string(" \n ");

    assert!(state.access_token_candidate_from_input().is_none());
    assert!(state.refresh_credentials_candidate_from_input().is_none());
    assert_eq!(state.access_token_input.as_str(), " \t ");
    assert_eq!(state.oauth_client_id_input.as_str(), " new-client ");
    assert_eq!(state.refresh_token_input.as_str(), " \n ");
    assert_eq!(
        state
            .pending_access_token_for_secret()
            .as_deref()
            .map(String::as_str),
        Some("pending-access")
    );
    let (client, refresh) = state
        .pending_oauth_credentials_for_secret()
        .expect("staged credentials");
    assert_eq!(client.as_str(), "pending-client");
    assert_eq!(refresh.as_str(), "pending-refresh");

    state.access_token_input = sensitive_string(" new-access ");
    state.refresh_token_input = sensitive_string(" new-refresh ");
    let access = state
        .access_token_candidate_from_input()
        .expect("valid access token");
    let (client, refresh) = state
        .refresh_credentials_candidate_from_input()
        .expect("valid refresh credentials");
    assert!(state.access_token_input.is_empty());
    assert!(state.oauth_client_id_input.is_empty());
    assert!(state.refresh_token_input.is_empty());
    let staged_access = state
        .pending_access_token_for_secret()
        .expect("pending access token");
    let staged_refresh = state
        .pending_oauth_credentials_for_secret()
        .expect("pending refresh credentials");
    let saved = state.oauth_credentials_for_secret();
    state.clear_pending_access_token();
    state.clear_pending_oauth_credentials();
    state.clear_access_token();

    assert_eq!(access.as_str(), "new-access");
    assert_eq!(client.as_str(), "new-client");
    assert_eq!(refresh.as_str(), "new-refresh");
    assert_eq!(staged_access.as_str(), access.as_str());
    assert_eq!(staged_refresh.0.as_str(), client.as_str());
    assert_eq!(staged_refresh.1.as_str(), refresh.as_str());
    assert_eq!(saved.0.as_str(), "saved-access");
    assert_eq!(saved.1.as_str(), "saved-client");
    assert_eq!(saved.2.as_str(), "saved-refresh");
    assert!(state.pending_access_token_for_secret().is_none());
    assert!(state.pending_oauth_credentials_for_secret().is_none());
    state.pending_oauth_client_id = sensitive_string("client-only");
    assert!(state.pending_oauth_credentials_for_secret().is_none());
}

fn fill_credential_inputs(state: &mut XFeedState) {
    state.access_token_input = sensitive_string("access-input");
    state.oauth_client_id_input = sensitive_string("client-input");
    state.refresh_token_input = sensitive_string("refresh-input");
    state.pending_access_token = sensitive_string("pending-access");
    state.pending_oauth_client_id = sensitive_string("pending-client");
    state.pending_refresh_token = sensitive_string("pending-refresh");
}

fn assert_credential_inputs_empty(state: &XFeedState) {
    assert!(state.access_token_input.is_empty());
    assert!(state.oauth_client_id_input.is_empty());
    assert!(state.refresh_token_input.is_empty());
    assert!(state.pending_access_token.is_empty());
    assert!(state.pending_oauth_client_id.is_empty());
    assert!(state.pending_refresh_token.is_empty());
}

#[test]
fn credential_commits_clear_inputs_and_invalidate_only_changed_credentials() {
    for direct in [true, false] {
        for changed in [false, true] {
            let (client, refresh) = if direct {
                ("", "")
            } else {
                ("client", "refresh")
            };
            let mut state = XFeedState::new(&[], "access", client, refresh);
            fill_credential_inputs(&mut state);
            state.connecting = true;
            state.token_refreshing = true;
            state.lists_loading = true;
            state.auth_user = Some(XAuthenticatedUser {
                id: "42".to_string(),
                username: "alice".to_string(),
                name: "Alice".to_string(),
            });
            state.lists.push(XListSummary {
                id: "10".to_string(),
                name: "Markets".to_string(),
                private: false,
                owner: XListOwnerKind::Owned,
            });
            let post = test_post("1", 1_000);
            state
                .author_profiles
                .insert(post.author_profile_key(), XAuthorProfile::from_post(&post));
            let sources = [
                XFeedSource::Following,
                XFeedSource::List {
                    id: "10".to_string(),
                    name: "Markets".to_string(),
                    private: false,
                },
                XFeedSource::List {
                    id: "11".to_string(),
                    name: "Private".to_string(),
                    private: true,
                },
            ];
            for (id, source) in sources.iter().enumerate() {
                let mut instance = XFeedInstance::new(id as u64, source.clone());
                instance.posts.push(post.clone());
                instance.last_error = Some("prior error".to_string());
                instance.last_refresh_ms = Some(1_000);
                state.instances.insert(id as u64, instance);
            }
            let request_id = state.begin_source_refresh(&XFeedSource::Following);
            state.set_source_rate_limit(&XFeedSource::Following, 10_000);
            let access = if changed { " new-access " } else { " access " };
            let committed = if direct {
                state.commit_access_token(access)
            } else {
                state.commit_oauth_credentials(access, " client ", " refresh ", Some(50_000))
            };

            assert_eq!(committed, changed);
            assert_credential_inputs_empty(&state);
            assert_eq!(state.access_token.as_str(), access.trim());
            assert_eq!(state.oauth_client_id.as_str(), client);
            assert_eq!(state.refresh_token.as_str(), refresh);
            assert_eq!(
                state.access_token_expires_at_ms,
                if direct { None } else { Some(50_000) }
            );
            assert_eq!(state.connect_request_id, u64::from(changed));
            assert_eq!(state.token_refresh_request_id, u64::from(changed));
            assert_eq!(state.lists_request_id, u64::from(changed));
            assert_eq!(state.refresh_request_id, request_id + u64::from(changed));
            assert_eq!(state.connecting, !changed);
            assert_eq!(state.token_refreshing, !changed);
            assert_eq!(state.lists_loading, !changed);
            assert_eq!(state.auth_user.is_some(), !changed);
            assert_eq!(state.lists.is_empty(), changed);
            assert_eq!(state.author_profiles.is_empty(), changed);
            assert_eq!(
                state.source_refresh_in_flight(&XFeedSource::Following),
                !changed
            );
            assert_eq!(
                state.source_rate_limited_until(&XFeedSource::Following, 0),
                if changed { None } else { Some(10_000) }
            );
            for (id, source) in sources.iter().enumerate() {
                let instance = &state.instances[&(id as u64)];
                let expected_source = if changed && source.is_private() {
                    &XFeedSource::Following
                } else {
                    source
                };
                assert_eq!(&instance.source, expected_source);
                assert_eq!(instance.posts.is_empty(), changed);
                assert_eq!(instance.last_error.is_none(), changed);
                assert_eq!(instance.last_refresh_ms.is_none(), changed);
            }
        }
    }
}

#[test]
fn clearing_credentials_preserves_generation_counts_with_or_without_saved_credentials() {
    for saved in ["", "access"] {
        let mut state = XFeedState::new(&[], saved, "", "");
        fill_credential_inputs(&mut state);
        state.connecting = true;
        state.token_refreshing = true;
        state.lists_loading = true;
        let request_id = state.begin_source_refresh(&XFeedSource::Following);
        state.set_source_rate_limit(&XFeedSource::Following, 10_000);

        state.clear_access_token();

        assert_credential_inputs_empty(&state);
        let increments = if saved.is_empty() { 1 } else { 2 };
        assert_eq!(state.connect_request_id, increments);
        assert_eq!(state.token_refresh_request_id, increments);
        assert_eq!(state.lists_request_id, increments);
        assert_eq!(state.refresh_request_id, request_id + increments);
        assert_eq!(state.connecting, saved.is_empty());
        assert_eq!(state.lists_loading, saved.is_empty());
        assert!(!state.token_refreshing);
        assert!(!state.source_refresh_in_flight(&XFeedSource::Following));
        assert_eq!(
            state.source_rate_limited_until(&XFeedSource::Following, 0),
            None
        );
        assert!(!state.has_access_token());
        assert!(!state.has_refresh_credentials());
        assert_eq!(state.status, Some(("X token cleared".to_string(), false)));
    }
}

#[test]
fn x_feed_instance_dedupes_and_sorts_posts() {
    let mut instance = XFeedInstance::new(0, XFeedSource::Following);
    let page = XFeedPage {
        source: XFeedSource::Following,
        posts: vec![
            test_post("1", 1_000),
            test_post("2", 2_000),
            test_post("1", 1_000),
        ],
        newest_id: Some("2".to_string()),
        rate_limited_until_ms: None,
    };

    instance.apply_page(&page, 3_000);

    assert_eq!(instance.posts.len(), 2);
    assert_eq!(instance.posts[0].id, "2");
    assert_eq!(instance.newest_seen_id(), Some("2"));
}

#[test]
fn x_feed_source_options_dedupe_lists() {
    let mut state = XFeedState::new(&[], "", "", "");
    state.lists = vec![
        XListSummary {
            id: "10".to_string(),
            name: "Macro".to_string(),
            private: false,
            owner: XListOwnerKind::Owned,
        },
        XListSummary {
            id: "10".to_string(),
            name: "Macro copy".to_string(),
            private: false,
            owner: XListOwnerKind::Followed,
        },
    ];

    let options = state.source_options();

    assert_eq!(options.len(), 2);
    assert!(matches!(options[0].source, XFeedSource::Following));
    assert_eq!(options[1].source.key(), "list:10");
}

#[test]
fn newest_seen_id_preserves_numeric_order_invalid_filtering_and_last_tie() {
    let mut instance = XFeedInstance::new(0, XFeedSource::Following);
    for id in ["invalid", "18446744073709551616", "-1", " 9"] {
        instance.posts.push(test_post(id, 1_000));
    }
    assert!(instance.newest_seen_id().is_none());
    for id in ["9", "10", "010"] {
        instance.posts.push(test_post(id, 1_000));
    }
    assert_eq!(instance.newest_seen_id(), Some("010"));
    instance.posts.push(test_post("18446744073709551615", 0));
    assert_eq!(instance.newest_seen_id(), Some("18446744073709551615"));
}

#[test]
fn source_options_keep_ascii_name_order_id_ties_and_first_sorted_duplicate() {
    let mut state = XFeedState::new(&[], "", "", "");
    state.lists = [
        ("2", "ALPHA", true),
        ("10", "alpha", false),
        ("2", "Alpha", false),
        ("4", "zulu", false),
        ("5", "Älpha", false),
        ("4", "Beta", true),
    ]
    .into_iter()
    .map(|(id, name, private)| XListSummary {
        id: id.to_string(),
        name: name.to_string(),
        private,
        owner: XListOwnerKind::Owned,
    })
    .collect();
    let original = state.lists.clone();

    let options = state.source_options();

    assert!(matches!(options[0].source, XFeedSource::Following));
    let sources = options[1..]
        .iter()
        .map(|option| &option.source)
        .collect::<Vec<_>>();
    let expected = [1, 0, 5, 4].map(|index| XFeedSource::List {
        id: original[index].id.clone(),
        name: original[index].name.clone(),
        private: original[index].private,
    });
    assert_eq!(sources, expected.iter().collect::<Vec<_>>());
    assert_eq!(state.lists, original);
}

fn test_post(id: &str, created_at_ms: u64) -> XFeedPost {
    XFeedPost {
        id: id.to_string(),
        author_id: Some("42".to_string()),
        author_name: "Alice".to_string(),
        author_username: "alice".to_string(),
        author_profile_image_url: Some("https://example.com/alice.jpg".to_string()),
        text: "hello".to_string(),
        created_at_ms,
        received_at_ms: created_at_ms,
        url: format!("https://x.com/alice/status/{id}"),
    }
}
