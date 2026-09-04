use serde_json::json;
use syncables::{apply_overlay, OverlayDocument};

fn overlay(actions: serde_json::Value) -> OverlayDocument {
    serde_json::from_value(json!({
        "overlay": "1.0.0",
        "info": { "title": "test", "version": "1.0.0" },
        "actions": actions
    }))
    .expect("valid overlay")
}

#[test]
fn deep_merges_an_update_onto_its_target() {
    let document = json!({ "components": { "schemas": { "Pet": { "type": "object" } } } });
    let result = apply_overlay(
        &document,
        &overlay(json!([{
            "target": "$.components",
            "update": { "paginationSchemes": { "byPage": { "type": "pageNumber" } } }
        }])),
    )
    .expect("overlay applies");

    assert_eq!(
        result["components"]["paginationSchemes"]["byPage"]["type"],
        json!("pageNumber")
    );
    // The merge is additive: what was already there survives.
    assert_eq!(
        result["components"]["schemas"]["Pet"]["type"],
        json!("object")
    );
}

#[test]
fn creates_missing_intermediate_objects() {
    let document = json!({});
    let result = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.components.schemas", "update": { "Pet": { "type": "object" } } }])),
    )
    .expect("overlay applies");
    assert_eq!(
        result["components"]["schemas"]["Pet"]["type"],
        json!("object")
    );
}

#[test]
fn removes_a_target() {
    let document = json!({ "components": { "schemas": { "Pet": {} } } });
    let result = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.components.schemas", "remove": true }])),
    )
    .expect("overlay applies");
    assert!(result["components"].get("schemas").is_none());
}

#[test]
fn rejects_targets_outside_the_supported_subset() {
    let document = json!({});
    let error = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.paths[*].get", "update": {} }])),
    )
    .expect_err("wildcard targets are unsupported");
    assert!(error.to_string().contains("unsupported overlay target"));
}

#[test]
fn applies_an_update_to_a_bracket_quoted_path_target() {
    // Real overlays (e.g. GitHub's crud-causality overlay) address a
    // specific operation this way, because the path template's slashes and
    // braces would otherwise be mangled by a plain dot-split.
    let document = json!({
        "paths": {
            "/repos/{owner}/{repo}/issues": {
                "get": { "summary": "List issues" }
            }
        }
    });
    let result = apply_overlay(
        &document,
        &overlay(json!([{
            "target": "$.paths['/repos/{owner}/{repo}/issues'].get",
            "update": { "x-crud": { "action": "list", "resource": "issue" } }
        }])),
    )
    .expect("overlay applies");

    assert_eq!(
        result["paths"]["/repos/{owner}/{repo}/issues"]["get"]["x-crud"]["action"],
        json!("list")
    );
    // Unrelated sibling data survives the merge.
    assert_eq!(
        result["paths"]["/repos/{owner}/{repo}/issues"]["get"]["summary"],
        json!("List issues")
    );
}

#[test]
fn bracket_target_key_may_be_double_quoted() {
    let document = json!({});
    let result = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.paths[\"/pets\"].get", "update": { "x-crud": {} } }])),
    )
    .expect("overlay applies");
    assert!(result["paths"]["/pets"]["get"]["x-crud"].is_object());
}

#[test]
fn rejects_an_unterminated_bracket() {
    let document = json!({});
    let error = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.paths['/pets", "update": {} }])),
    )
    .expect_err("an unterminated bracket is unsupported");
    assert!(error.to_string().contains("unsupported overlay target"));
}

#[test]
fn rejects_an_unquoted_bracket_key() {
    let document = json!({});
    let error = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$.paths[pets].get", "update": {} }])),
    )
    .expect_err("unquoted bracket keys are unsupported");
    assert!(error.to_string().contains("unsupported overlay target"));
}

#[test]
fn refuses_to_remove_the_document_root() {
    let document = json!({});
    let error = apply_overlay(
        &document,
        &overlay(json!([{ "target": "$", "remove": true }])),
    )
    .expect_err("the root cannot be removed");
    assert!(error.to_string().contains("document root"));
}
