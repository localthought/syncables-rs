use serde_json::json;
use syncables::resolve_refs;

#[test]
fn inlines_local_pointers() {
    let document = json!({
        "components": { "schemas": { "Pet": { "type": "object" } } },
        "paths": { "/pets": { "get": { "schema": { "$ref": "#/components/schemas/Pet" } } } }
    });
    let resolved = resolve_refs(&document);
    assert_eq!(
        resolved["paths"]["/pets"]["get"]["schema"],
        json!({ "type": "object" })
    );
}

#[test]
fn shares_one_result_across_diamond_refs() {
    let document = json!({
        "components": { "schemas": { "Pet": { "type": "object" } } },
        "a": { "$ref": "#/components/schemas/Pet" },
        "b": { "$ref": "#/components/schemas/Pet" }
    });
    let resolved = resolve_refs(&document);
    assert_eq!(resolved["a"], resolved["b"]);
    assert_eq!(resolved["a"], json!({ "type": "object" }));
}

#[test]
fn leaves_a_cycle_unresolved_rather_than_recursing_forever() {
    let document = json!({
        "components": { "schemas": { "Node": { "child": { "$ref": "#/components/schemas/Node" } } } }
    });
    let resolved = resolve_refs(&document);
    assert!(resolved["components"]["schemas"]["Node"]["child"].is_object());
}

#[test]
fn leaves_non_local_refs_alone() {
    let document = json!({ "a": { "$ref": "./other.yaml#/Thing" } });
    let resolved = resolve_refs(&document);
    assert_eq!(resolved["a"]["$ref"], json!("./other.yaml#/Thing"));
}

#[test]
fn resolves_the_pets_fixture_all_of_branches() {
    let resolved = resolve_refs(&crate::pets::pets_document());
    let pet = &resolved["components"]["schemas"]["Pet"]["allOf"][0];
    assert_eq!(pet["type"], json!("object"));
    assert!(pet["properties"]["name"].is_object());
}
