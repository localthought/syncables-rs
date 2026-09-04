use serde_json::json;
use syncables::pagination::request_builder::{build_query, next_cursor, PageCursor};
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
