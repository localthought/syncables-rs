use serde_json::json;
use syncables::{resolve_effective_scheme, OpenApiDocument};

fn document(value: serde_json::Value) -> OpenApiDocument {
    serde_json::from_value(value).expect("valid document")
}

fn with_schemes(schemes: serde_json::Value, path_item: serde_json::Value) -> OpenApiDocument {
    document(json!({
        "openapi": "3.0.0",
        "info": { "title": "t", "version": "1" },
        "paths": { "/things": path_item },
        "components": { "paginationSchemes": schemes }
    }))
}

#[test]
fn auto_detects_by_declared_query_parameters() {
    let doc = with_schemes(
        json!({ "byPage": {
            "type": "pageNumber",
            "request": { "queryParameters": { "page": { "role": "page" } } }
        } }),
        json!({ "get": {
            "parameters": [{ "name": "page", "in": "query" }],
            "responses": {}
        } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    let resolved = resolve_effective_scheme(&doc, operation).expect("a scheme applies");
    assert_eq!(resolved.scheme_name, "byPage");
}

#[test]
fn a_dimension_with_no_declared_fields_never_matches_vacuously() {
    let doc = with_schemes(
        json!({ "empty": { "type": "pageNumber", "request": { "queryParameters": {} } } }),
        json!({ "get": { "responses": {} } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    assert!(resolve_effective_scheme(&doc, operation).is_none());
}

#[test]
fn auto_detect_false_opts_a_scheme_out() {
    let doc = with_schemes(
        json!({ "byPage": {
            "type": "pageNumber",
            "autoDetect": false,
            "request": { "queryParameters": { "page": { "role": "page" } } }
        } }),
        json!({ "get": {
            "parameters": [{ "name": "page", "in": "query" }],
            "responses": {}
        } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    assert!(resolve_effective_scheme(&doc, operation).is_none());
}

#[test]
fn an_invalid_scheme_is_skipped_rather_than_failing_the_document() {
    let doc = with_schemes(
        json!({
            "broken": {
                "type": "offset",
                "request": { "queryParameters": { "page": { "role": "page" } } }
            },
            "byPage": {
                "type": "pageNumber",
                "request": { "queryParameters": { "page": { "role": "page" } } }
            }
        }),
        json!({ "get": {
            "parameters": [{ "name": "page", "in": "query" }],
            "responses": {}
        } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    let resolved = resolve_effective_scheme(&doc, operation).expect("a scheme applies");
    assert_eq!(resolved.scheme_name, "byPage");
}

#[test]
fn an_explicit_x_pagination_takes_priority_over_auto_detection() {
    let doc = with_schemes(
        json!({
            "byPage": {
                "type": "pageNumber",
                "request": { "queryParameters": { "page": { "role": "page" } } }
            },
            "byToken": {
                "type": "pageToken",
                "request": { "queryParameters": { "cursor": { "role": "cursor" } } }
            }
        }),
        json!({ "get": {
            "parameters": [{ "name": "page", "in": "query" }],
            "responses": {},
            "x-pagination": [{ "scheme": "byToken" }]
        } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    let resolved = resolve_effective_scheme(&doc, operation).expect("a scheme applies");
    assert_eq!(resolved.scheme_name, "byToken");
}

#[test]
fn explicit_overrides_are_deep_merged_onto_the_named_scheme() {
    let doc = with_schemes(
        json!({ "byPage": {
            "type": "pageNumber",
            "request": { "queryParameters": { "page": { "role": "page" } } }
        } }),
        json!({ "get": {
            "responses": {},
            "x-pagination": [{
                "scheme": "byPage",
                "overrides": { "request": { "queryParameters": { "limit": { "role": "pageSize" } } } }
            }]
        } }),
    );
    let operation = doc.paths["/things"].get.as_ref().expect("a get operation");
    let resolved = resolve_effective_scheme(&doc, operation).expect("a scheme applies");

    let params = resolved
        .scheme
        .request
        .as_ref()
        .and_then(|r| r.query_parameters.as_ref())
        .expect("query parameters survive the merge");
    assert!(params.contains_key("page"), "base field survives");
    assert!(params.contains_key("limit"), "override is merged in");
}
