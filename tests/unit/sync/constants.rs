use std::collections::BTreeMap;

use serde_json::json;
use syncables::{bind_url, discover_resource_model, validate_constants, Error, OpenApiDocument};

/// The GitHub crud-causality overlay's own example: `owner`/`repo` are
/// declared path parameters no collection can enumerate, `issue_number` is
/// resolved from the parent `issues` collection.
fn github_like_document() -> OpenApiDocument {
    serde_json::from_value(json!({
        "paths": {
            "/repos/{owner}/{repo}/issues": {
                "get": {
                    "parameters": [
                        { "name": "owner", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "repo", "in": "path", "required": true, "schema": { "type": "string" } }
                    ]
                }
            },
            "/repos/{owner}/{repo}/issues/{issue_number}/comments": {
                "get": {
                    "parameters": [
                        { "name": "owner", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "repo", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "issue_number", "in": "path", "required": true, "schema": { "type": "integer" } }
                    ]
                }
            }
        },
        "components": {
            "crudResources": {
                "issue": {
                    "identity": {
                        "urlTemplate": "/repos/{owner}/{repo}/issues/{issue_number}",
                        "bindings": { "issue_number": { "field": "number" } }
                    },
                    "collections": {
                        "issues": { "urlTemplate": "/repos/{owner}/{repo}/issues" }
                    }
                },
                "issueComment": {
                    "identity": {
                        "urlTemplate": "/repos/{owner}/{repo}/issues/comments/{comment_id}",
                        "bindings": { "comment_id": { "field": "id" } }
                    },
                    "collections": {
                        "issueComments": {
                            "urlTemplate": "/repos/{owner}/{repo}/issues/{issue_number}/comments"
                        }
                    }
                }
            }
        }
    }))
    .expect("valid document")
}

fn constants(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn accepts_constants_that_resolve_every_path_variable() {
    let document = github_like_document();
    let model = discover_resource_model(&document).expect("model derives");
    let constants = constants(&[("owner", "localthought"), ("repo", "test-repo-1")]);
    validate_constants(&document, &model, &constants).expect("owner/repo cover every gap");
}

#[test]
fn a_constant_naming_an_undeclared_parameter_is_an_error() {
    let document = github_like_document();
    let model = discover_resource_model(&document).expect("model derives");
    // A typo'd key ("repository" instead of "repo") must not be silently
    // ignored — that would leave `repo` unbound and widen the sync.
    let constants = constants(&[("owner", "localthought"), ("repository", "test-repo-1")]);
    let error = validate_constants(&document, &model, &constants)
        .expect_err("repository is not a declared parameter");
    assert!(matches!(error, Error::UnknownConstant(key) if key == "repository"));
}

#[test]
fn a_missing_constant_leaves_a_path_variable_unbound() {
    let document = github_like_document();
    let model = discover_resource_model(&document).expect("model derives");
    // `repo` is never supplied.
    let constants = constants(&[("owner", "localthought")]);
    let error = validate_constants(&document, &model, &constants)
        .expect_err("repo is bound by neither a constant nor a provider");
    assert!(matches!(error, Error::UnboundContextParam(param) if param == "repo"));
}

#[test]
fn a_nested_collections_parent_provided_variable_needs_no_constant() {
    let document = github_like_document();
    let model = discover_resource_model(&document).expect("model derives");
    // `issue_number` (issueComments' context param) is resolved by the
    // issues collection's own identity binding, not a constant.
    let constants = constants(&[("owner", "localthought"), ("repo", "test-repo-1")]);
    validate_constants(&document, &model, &constants).expect("issue_number has a provider");
}

#[test]
fn an_items_own_identity_binding_needs_no_constant() {
    let document = github_like_document();
    let model = discover_resource_model(&document).expect("model derives");
    // issueComments' item_url has {comment_id}, which is issueComment's own
    // identity binding — it must not be treated as an unresolved context
    // variable even though no constant or provider supplies it.
    let constants = constants(&[("owner", "localthought"), ("repo", "test-repo-1")]);
    validate_constants(&document, &model, &constants).expect("comment_id is self-bound");
}

#[test]
fn binds_a_url_template_with_percent_encoding() {
    let values = constants(&[("owner", "local thought"), ("repo", "test-repo-1")]);
    let bound = bind_url("/repos/{owner}/{repo}/issues", &values).expect("all variables bound");
    assert_eq!(bound, "/repos/local%20thought/test-repo-1/issues");
}

#[test]
fn binding_a_template_with_an_unbound_variable_is_an_error() {
    let values = constants(&[("owner", "localthought")]);
    let error = bind_url("/repos/{owner}/{repo}/issues", &values).expect_err("repo has no value");
    assert!(matches!(error, Error::UnboundContextParam(param) if param == "repo"));
}

#[test]
fn binding_a_template_with_no_variables_is_unchanged() {
    let values = BTreeMap::new();
    assert_eq!(
        bind_url("/healthz", &values).expect("no variables to bind"),
        "/healthz"
    );
}
