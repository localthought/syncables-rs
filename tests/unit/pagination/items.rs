use serde_json::json;
use syncables::pagination::items::{effective_properties, item_schema_for, locate_items_field};
use syncables::{PaginationSchemeObject, SchemaObject};

fn schema(value: serde_json::Value) -> SchemaObject {
    serde_json::from_value(value).expect("valid schema")
}

fn scheme(value: serde_json::Value) -> PaginationSchemeObject {
    serde_json::from_value(value).expect("valid scheme")
}

#[test]
fn flattens_all_of_branches_into_one_property_map() {
    let properties = effective_properties(Some(&schema(json!({
        "allOf": [
            { "type": "object", "properties": { "total": { "type": "integer" } } },
            { "type": "object", "properties": { "items": { "type": "array" } } }
        ]
    }))));
    assert_eq!(properties.len(), 2);
    assert!(properties.contains_key("total"));
    assert!(properties.contains_key("items"));
}

#[test]
fn picks_the_first_array_property_not_claimed_as_metadata() {
    let response = schema(json!({
        "type": "object",
        "properties": {
            "pagination": { "type": "object" },
            "data": { "type": "array", "items": { "type": "object" } }
        }
    }));
    assert_eq!(
        locate_items_field(Some(&response), None).as_deref(),
        Some("data")
    );
}

#[test]
fn excludes_fields_the_scheme_claims_for_metadata() {
    let response = schema(json!({
        "type": "object",
        "properties": {
            "links": { "type": "array" },
            "results": { "type": "array" }
        }
    }));
    let scheme = scheme(json!({
        "type": "nextLink",
        "response": { "bodyFields": { "links.next": { "role": "nextLink" } } }
    }));
    assert_eq!(
        locate_items_field(Some(&response), Some(&scheme)).as_deref(),
        Some("results")
    );
}

#[test]
fn unwraps_a_bare_array_response_schema() {
    let response = schema(json!({ "type": "array", "items": { "type": "string" } }));
    let item = item_schema_for(Some(&response), None).expect("has an item schema");
    assert_eq!(item.schema_type.as_deref(), Some("string"));
}

#[test]
fn unwraps_an_enveloped_response_schema() {
    let response = schema(json!({
        "type": "object",
        "properties": { "data": { "type": "array", "items": { "type": "string" } } }
    }));
    let item = item_schema_for(Some(&response), None).expect("has an item schema");
    assert_eq!(item.schema_type.as_deref(), Some("string"));
}
