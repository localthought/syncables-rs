use syncables::resources::discover::UpdateMethod;
use syncables::{discover_resources, resolve_refs, OpenApiDocument};

fn document_from(value: serde_json::Value) -> OpenApiDocument {
    serde_json::from_value(resolve_refs(&value)).expect("valid document")
}

#[test]
fn pairs_a_collection_path_with_its_item_path() {
    let document = document_from(crate::pets::pets_document());
    let resources = discover_resources(&document.paths);

    assert_eq!(resources.len(), 1);
    let pets = &resources[0];
    assert_eq!(pets.collection_path, "/pets");
    assert_eq!(pets.item_path, "/pets/{petId}");
    assert_eq!(pets.item_param, "petId");
    assert_eq!(pets.update_method, UpdateMethod::Put);
}

#[test]
fn omits_paths_with_no_item_path() {
    let document = document_from(crate::pets::pets_document());
    let resources = discover_resources(&document.paths);
    assert!(resources.iter().all(|r| r.collection_path != "/health"));
}

#[test]
fn falls_back_to_patch_when_the_item_path_has_no_put() {
    let document = document_from(serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "t", "version": "1" },
        "paths": {
            "/issues": { "get": { "responses": {} } },
            "/issues/{number}": { "get": { "responses": {} }, "patch": { "responses": {} } }
        }
    }));
    let resources = discover_resources(&document.paths);
    assert_eq!(resources[0].update_method, UpdateMethod::Patch);
    assert_eq!(resources[0].update_method.as_str(), "PATCH");
}

#[test]
fn only_pairs_direct_children() {
    let document = document_from(serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "t", "version": "1" },
        "paths": {
            "/pets": { "get": { "responses": {} } },
            "/pets/{petId}/toys/{toyId}": { "get": { "responses": {} } }
        }
    }));
    assert!(discover_resources(&document.paths).is_empty());
}
