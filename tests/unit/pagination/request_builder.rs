use serde_json::json;
use syncables::pagination::request_builder::{
    build_query, next_cursor, next_step, PageCursor, PageStep, MAX_PAGES,
};
use syncables::{PaginationResponseState, PaginationSchemeObject};

fn scheme(value: serde_json::Value) -> PaginationSchemeObject {
    serde_json::from_value(value).expect("valid scheme")
}

fn by_offset() -> PaginationSchemeObject {
    scheme(json!({
        "type": "pageNumber",
        "request": { "queryParameters": {
            "offset": { "role": "offset" },
            "limit": { "role": "pageSize" }
        } }
    }))
}

fn by_page() -> PaginationSchemeObject {
    scheme(json!({
        "type": "pageNumber",
        "request": { "queryParameters": { "page": { "role": "page" } } }
    }))
}

#[test]
fn builds_offset_and_page_size_parameters() {
    let query = build_query(
        &by_offset(),
        &PageCursor {
            offset: Some(20),
            ..PageCursor::default()
        },
        Some(10),
    );
    assert_eq!(query.get("offset").map(String::as_str), Some("20"));
    assert_eq!(query.get("limit").map(String::as_str), Some("10"));
}

#[test]
fn defaults_offset_to_zero_and_page_to_one() {
    assert_eq!(
        build_query(&by_offset(), &PageCursor::default(), None)
            .get("offset")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        build_query(&by_page(), &PageCursor::default(), None)
            .get("page")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn sends_a_token_under_both_page_token_and_cursor_roles() {
    let scheme = scheme(json!({
        "type": "pageToken",
        "request": { "queryParameters": {
            "page_token": { "role": "pageToken" },
            "cursor": { "role": "cursor" }
        } }
    }));
    let query = build_query(
        &scheme,
        &PageCursor {
            page_token: Some("abc".into()),
            ..PageCursor::default()
        },
        None,
    );
    assert_eq!(query.get("page_token").map(String::as_str), Some("abc"));
    assert_eq!(query.get("cursor").map(String::as_str), Some("abc"));
}

#[test]
fn stops_when_the_response_says_there_is_no_next_page() {
    let state = PaginationResponseState::default();
    assert!(next_cursor(&by_offset(), &PageCursor::default(), &state, 10).is_none());
}

#[test]
fn advances_an_offset_by_the_items_returned() {
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let next = next_cursor(&by_offset(), &PageCursor::default(), &state, 10)
        .expect("there is a next page");
    assert_eq!(next.offset, Some(10));
}

#[test]
fn increments_a_page_number() {
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let next =
        next_cursor(&by_page(), &PageCursor::default(), &state, 10).expect("there is a next page");
    assert_eq!(next.page, Some(2));
}

#[test]
fn yields_no_cursor_for_a_next_link_scheme() {
    // The caller should follow `state.next_link` directly instead.
    let scheme = scheme(json!({
        "type": "nextLink",
        "response": { "bodyFields": { "next": { "role": "nextLink" } } }
    }));
    let state = PaginationResponseState {
        next_link: Some("https://example.com/next".into()),
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    assert!(next_cursor(&scheme, &PageCursor::default(), &state, 10).is_none());
}

#[test]
fn next_step_stops_when_there_is_no_next_page() {
    let state = PaginationResponseState::default();
    assert_eq!(
        next_step(&by_offset(), &PageCursor::default(), &state, 10, 1),
        PageStep::Done
    );
}

#[test]
fn next_step_advances_an_offset_scheme() {
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let step = next_step(&by_offset(), &PageCursor::default(), &state, 10, 1);
    assert_eq!(
        step,
        PageStep::NextPage(PageCursor {
            offset: Some(10),
            ..PageCursor::default()
        })
    );
}

#[test]
fn next_step_follows_a_next_link_url_directly_rather_than_a_cursor() {
    let scheme = scheme(json!({
        "type": "nextLink",
        "response": { "headers": { "Link": { "role": "nextLink" } } }
    }));
    let state = PaginationResponseState {
        next_link: Some("https://api.github.com/repos/o/r/issues?page=2".into()),
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let step = next_step(&scheme, &PageCursor::default(), &state, 1, 1);
    assert_eq!(
        step,
        PageStep::FollowLink("https://api.github.com/repos/o/r/issues?page=2".into())
    );
}

#[test]
fn next_step_stops_a_next_link_scheme_with_no_link_even_if_has_next_page_is_set() {
    let scheme = scheme(json!({
        "type": "nextLink",
        "response": { "headers": { "Link": { "role": "nextLink" } } }
    }));
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    assert_eq!(
        next_step(&scheme, &PageCursor::default(), &state, 1, 1),
        PageStep::Done
    );
}

#[test]
fn next_step_stops_once_the_page_cap_is_reached_even_though_another_page_exists() {
    // A misconfigured `Link` header (or any response that always claims
    // another page) must not spin forever.
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let step = next_step(&by_offset(), &PageCursor::default(), &state, 10, MAX_PAGES);
    assert_eq!(step, PageStep::Done);
}

#[test]
fn next_step_does_not_stop_one_page_short_of_the_cap() {
    let state = PaginationResponseState {
        has_next_page: true,
        ..PaginationResponseState::default()
    };
    let step = next_step(
        &by_offset(),
        &PageCursor::default(),
        &state,
        10,
        MAX_PAGES - 1,
    );
    assert_ne!(step, PageStep::Done);
}
