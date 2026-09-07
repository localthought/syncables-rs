use serde_json::json;
use syncables::{OpenApiDocument, SchemaObject, SchemaType};

fn schema(value: serde_json::Value) -> SchemaObject {
    serde_json::from_value(value).expect("valid schema")
}

#[test]
fn parses_a_single_type_string() {
    let schema = schema(json!({ "type": "string" }));
    assert_eq!(
        schema.schema_type,
        Some(SchemaType::Single("string".to_string()))
    );
}

#[test]
fn parses_a_nullable_type_array() {
    let schema = schema(json!({ "type": ["string", "null"] }));
    assert_eq!(
        schema.schema_type,
        Some(SchemaType::Multiple(vec![
            "string".to_string(),
            "null".to_string()
        ]))
    );
}

#[test]
fn contains_checks_both_forms() {
    assert!(SchemaType::Single("string".to_string()).contains("string"));
    assert!(!SchemaType::Single("string".to_string()).contains("null"));

    let nullable = SchemaType::Multiple(vec!["string".to_string(), "null".to_string()]);
    assert!(nullable.contains("string"));
    assert!(nullable.contains("null"));
    assert!(!nullable.contains("integer"));
}

#[test]
fn primary_picks_the_first_non_null_entry() {
    assert_eq!(
        SchemaType::Single("string".to_string()).primary(),
        Some("string")
    );
    assert_eq!(
        SchemaType::Multiple(vec!["string".to_string(), "null".to_string()]).primary(),
        Some("string")
    );
    assert_eq!(
        SchemaType::Multiple(vec!["null".to_string(), "string".to_string()]).primary(),
        Some("string")
    );
}

#[test]
fn primary_is_none_when_every_entry_is_null() {
    assert_eq!(
        SchemaType::Multiple(vec!["null".to_string()]).primary(),
        None
    );
    assert_eq!(SchemaType::Multiple(vec![]).primary(), None);
}

/// Reproduces localthought/syncables-rs#14: a document using JSON Schema
/// 2020-12's nullable `type: [T, "null"]` array form — as GitHub's own
/// OpenAPI document does for `issue.body` and `issue.state_reason` — used
/// to fail `load_open_api_document`/`serde_json` parsing entirely.
#[test]
fn a_document_with_a_nullable_array_type_schema_parses() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "info": { "title": "Widgets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "issue": {
                    "type": "object",
                    "properties": {
                        "body": { "type": ["string", "null"] },
                        "state_reason": { "type": ["string", "null"] }
                    }
                }
            }
        }
    }))
    .expect("a nullable array type schema parses");

    let issue = &document
        .components
        .expect("components")
        .schemas
        .expect("schemas")["issue"];
    let body = &issue.properties.as_ref().expect("properties")["body"];
    assert_eq!(
        body.schema_type.as_ref().and_then(SchemaType::primary),
        Some("string")
    );
    assert!(body.schema_type.as_ref().unwrap().contains("null"));
}
