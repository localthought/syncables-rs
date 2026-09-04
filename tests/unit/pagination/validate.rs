use serde_json::json;
use syncables::{validate_pagination_scheme, PaginationSchemeObject};

fn scheme(value: serde_json::Value) -> PaginationSchemeObject {
    serde_json::from_value(value).expect("parses leniently")
}

#[test]
fn accepts_a_well_formed_scheme() {
    let errors = validate_pagination_scheme(
        "byPage",
        &scheme(json!({
            "type": "pageNumber",
            "request": { "queryParameters": { "page": { "role": "page" } } }
        })),
    );
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn rejects_a_type_outside_the_spec() {
    // Giphy's overlay originally declared `type: offset`, which is not a
    // valid scheme type.
    let errors = validate_pagination_scheme(
        "byOffset",
        &scheme(json!({ "type": "offset", "request": { "queryParameters": {} } })),
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("paginationSchemes.byOffset.type"));
}

#[test]
fn requires_at_least_one_of_request_or_response() {
    let errors = validate_pagination_scheme("bare", &scheme(json!({ "type": "pageToken" })));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("at least one of"));
}

#[test]
fn rejects_unknown_request_and_response_roles() {
    let errors = validate_pagination_scheme(
        "odd",
        &scheme(json!({
            "type": "pageNumber",
            "request": { "queryParameters": { "p": { "role": "nope" } } },
            "response": { "bodyFields": { "n": { "role": "alsoNope" } } }
        })),
    );
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("not a valid request role"));
    assert!(errors[1].contains("not a valid response role"));
}

#[test]
fn allows_extension_roles() {
    let errors = validate_pagination_scheme(
        "ext",
        &scheme(json!({
            "type": "pageNumber",
            "request": { "queryParameters": { "p": { "role": "x-custom" } } }
        })),
    );
    assert!(errors.is_empty(), "{errors:?}");
}
