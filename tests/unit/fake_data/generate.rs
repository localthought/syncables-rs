use serde_json::json;
use syncables::{generate_from_schema, SchemaObject};

fn schema(value: serde_json::Value) -> SchemaObject {
    serde_json::from_value(value).expect("valid schema")
}

#[test]
fn prefers_the_schemas_own_example() {
    let generated = generate_from_schema(Some(&schema(
        json!({ "type": "string", "example": "Milo" }),
    )));
    assert_eq!(generated, json!("Milo"));
}

#[test]
fn prefers_the_first_enum_value_next() {
    let generated = generate_from_schema(Some(&schema(json!({ "enum": ["cat", "dog"] }))));
    assert_eq!(generated, json!("cat"));
}

#[test]
fn merges_all_of_branches() {
    let generated = generate_from_schema(Some(&schema(json!({
        "allOf": [
            { "type": "object", "properties": { "name": { "type": "string", "example": "Milo" } } },
            { "type": "object", "properties": { "tag": { "type": "string", "example": "cat" } } }
        ]
    }))));
    assert_eq!(generated, json!({ "name": "Milo", "tag": "cat" }));
}

#[test]
fn takes_the_first_branch_of_one_of_and_any_of() {
    let one_of = generate_from_schema(Some(&schema(json!({
        "oneOf": [{ "type": "boolean" }, { "type": "string" }]
    }))));
    assert_eq!(one_of, json!(true));

    let any_of = generate_from_schema(Some(&schema(json!({
        "anyOf": [{ "type": "integer", "minimum": 5.0 }, { "type": "string" }]
    }))));
    assert_eq!(any_of, json!(5.0));
}

#[test]
fn synthesizes_known_string_formats() {
    for (format, expected) in [
        ("date-time", "1970-01-01T00:00:00.000Z"),
        ("date", "1970-01-01"),
        ("uuid", "00000000-0000-4000-8000-000000000000"),
        ("email", "user@example.com"),
        ("uri", "https://example.com"),
    ] {
        let generated =
            generate_from_schema(Some(&schema(json!({ "type": "string", "format": format }))));
        assert_eq!(generated, json!(expected), "format {format}");
    }
}

#[test]
fn wraps_an_array_schema_around_one_generated_item() {
    let generated = generate_from_schema(Some(&schema(json!({
        "type": "array",
        "items": { "type": "boolean" }
    }))));
    assert_eq!(generated, json!([true]));
}

#[test]
fn generates_null_for_a_missing_schema() {
    assert_eq!(generate_from_schema(None), json!(null));
}

#[test]
fn generates_from_a_nullable_array_type_schema() {
    let generated = generate_from_schema(Some(&schema(json!({ "type": ["string", "null"] }))));
    assert!(generated.is_string());
}
