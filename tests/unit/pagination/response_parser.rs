use indexmap::IndexMap;
use serde_json::json;
use syncables::pagination::response_parser::{
    parse_link_header, parse_pagination_state, read_nested_field, set_nested_field,
};
use syncables::PaginationSchemeObject;

fn scheme(value: serde_json::Value) -> PaginationSchemeObject {
    serde_json::from_value(value).expect("valid scheme")
}

fn headers(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn reads_a_dotted_path_out_of_a_body() {
    let body = json!({ "pagination": { "total_count": 42 } });
    assert_eq!(
        read_nested_field(&body, "pagination.total_count"),
        Some(&json!(42))
    );
    assert!(read_nested_field(&body, "pagination.missing").is_none());
    assert!(read_nested_field(&body, "nope.nope").is_none());
}

#[test]
fn writes_a_dotted_path_creating_intermediates() {
    let mut body = serde_json::Map::new();
    set_nested_field(&mut body, "pagination.total_count", json!(7));
    assert_eq!(body["pagination"]["total_count"], json!(7));
}

#[test]
fn extracts_the_next_url_from_a_link_header() {
    let header = "<https://api.example.com/items?page=2>; rel=\"next\", \
                  <https://api.example.com/items?page=1>; rel=\"prev\"";
    assert_eq!(
        parse_link_header(header).as_deref(),
        Some("https://api.example.com/items?page=2")
    );
}

#[test]
fn tolerates_an_unquoted_rel_and_returns_none_without_a_next() {
    assert_eq!(
        parse_link_header("<https://example.com/2>; rel=next").as_deref(),
        Some("https://example.com/2")
    );
    assert!(parse_link_header("<https://example.com/1>; rel=\"prev\"").is_none());
    assert!(parse_link_header("").is_none());
}

#[test]
fn parses_body_fields_by_role() {
    let scheme = scheme(json!({
        "type": "pageNumber",
        "response": { "bodyFields": {
            "pagination.total_count": { "role": "totalCount" },
            "pagination.count": { "role": "pageSize" }
        } }
    }));
    let body = json!({ "pagination": { "total_count": 50, "count": 25 } });
    let state = parse_pagination_state(&scheme, &body, &IndexMap::new(), Some(25));

    assert_eq!(state.total_count, Some(50.0));
    assert_eq!(state.page_size, Some(25.0));
    // 25 of 50 fetched so far, so another page exists.
    assert!(state.has_next_page);
}

#[test]
fn a_total_count_already_reached_ends_the_traversal() {
    let scheme = scheme(json!({
        "type": "pageNumber",
        "response": { "bodyFields": { "total": { "role": "totalCount" } } }
    }));
    let state =
        parse_pagination_state(&scheme, &json!({ "total": 30 }), &IndexMap::new(), Some(30));
    assert!(!state.has_next_page);
}

#[test]
fn a_next_link_header_is_a_strong_signal_of_another_page() {
    let scheme = scheme(json!({
        "type": "pageNumber",
        "response": { "headers": { "Link": { "role": "nextLink" } } }
    }));
    let state = parse_pagination_state(
        &scheme,
        &json!({}),
        &headers(&[("link", "<https://example.com/2>; rel=\"next\"")]),
        None,
    );
    assert_eq!(state.next_link.as_deref(), Some("https://example.com/2"));
    assert!(state.has_next_page);
}

#[test]
fn compares_current_page_against_total_pages() {
    let scheme = scheme(json!({
        "type": "pageNumber",
        "response": { "bodyFields": {
            "page": { "role": "currentPage" },
            "pages": { "role": "totalPages" }
        } }
    }));
    let state = parse_pagination_state(
        &scheme,
        &json!({ "page": 1, "pages": 3 }),
        &IndexMap::new(),
        None,
    );
    assert!(state.has_next_page);

    let last = parse_pagination_state(
        &scheme,
        &json!({ "page": 3, "pages": 3 }),
        &IndexMap::new(),
        None,
    );
    assert!(!last.has_next_page);
}

#[test]
fn treats_an_empty_token_as_no_token() {
    let scheme = scheme(json!({
        "type": "pageToken",
        "response": { "bodyFields": { "next": { "role": "nextPageToken" } } }
    }));
    let state = parse_pagination_state(&scheme, &json!({ "next": "" }), &IndexMap::new(), None);
    assert!(state.next_page_token.is_none());
    assert!(!state.has_next_page);
}
