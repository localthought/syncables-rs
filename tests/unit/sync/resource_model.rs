use serde_json::json;
use syncables::{
    crud_operation, discover_resource_model, CollectionMembership, CrudAction, Error,
    OpenApiDocument, SchemaType,
};

/// A document shaped like the GitHub crud-causality overlay's own example:
/// an `issue` resource addressed by `number` (not the payload's `id`), with
/// a nested `issueComment` resource whose collection is scoped under the
/// parent issue but whose item is addressed at a non-nested path.
fn github_like_document() -> OpenApiDocument {
    serde_json::from_value(json!({
        "openapi": "3.0.3",
        "info": { "title": "test", "version": "1.0.0" },
        "paths": {
            "/repos/{owner}/{repo}/issues": {
                "post": {
                    "x-crud": {
                        "action": "create",
                        "resource": "issue",
                        "url": { "source": "template" },
                        "addedFields": {
                            "number": {
                                "schema": { "type": "integer" },
                                "source": "server",
                                "description": "Server-assigned issue number."
                            }
                        },
                        "memberOf": ["issues"]
                    }
                },
                "get": {
                    "x-crud": { "action": "list", "resource": "issue", "collection": "issues" }
                }
            },
            "/repos/{owner}/{repo}/issues/{issue_number}": {
                "get": {
                    "x-crud": { "action": "read", "resource": "issue" }
                },
                "patch": {
                    "x-crud": {
                        "action": "update",
                        "mode": "patch",
                        "patchFormat": "merge",
                        "resource": "issue"
                    }
                }
            },
            "/repos/{owner}/{repo}/issues/{issue_number}/comments": {
                "get": {
                    "x-crud": {
                        "action": "list",
                        "resource": "issueComment",
                        "collection": "issueComments"
                    }
                }
            },
            "/repos/{owner}/{repo}/issues/comments/{comment_id}": {
                "delete": {
                    "x-crud": {
                        "action": "delete",
                        "resource": "issueComment",
                        "removesFrom": "*"
                    }
                }
            }
        },
        "components": {
            "crudResources": {
                "issue": {
                    "description": "An issue in a repository.",
                    "identity": {
                        "urlTemplate": "/repos/{owner}/{repo}/issues/{issue_number}",
                        "bindings": { "issue_number": { "field": "number" } }
                    },
                    "collections": {
                        "issues": {
                            "urlTemplate": "/repos/{owner}/{repo}/issues",
                            "x-list-query": { "state": "all" }
                        }
                    }
                },
                "issueComment": {
                    "description": "A comment on an issue.",
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

#[test]
fn errors_when_the_document_declares_no_crud_resources() {
    let document = OpenApiDocument::default();
    let error = discover_resource_model(&document).expect_err("no crudResources declared");
    assert!(matches!(error, Error::NoCrudResources));
}

#[test]
fn discovers_a_collection_per_resource() {
    let model = discover_resource_model(&github_like_document()).expect("model derives");
    assert_eq!(model.collections.len(), 2);

    let issues = model.by_name("issues").expect("issues collection");
    assert_eq!(issues.resource, "issue");
    assert_eq!(issues.collection_url, "/repos/{owner}/{repo}/issues");
    assert_eq!(
        issues.item_url,
        "/repos/{owner}/{repo}/issues/{issue_number}"
    );
    // The URL identity is `number`, not the payload's own `id`.
    assert_eq!(issues.id_field, "number");
    assert_eq!(
        issues.context_params,
        vec!["owner".to_string(), "repo".to_string()]
    );
    assert_eq!(issues.list_query.get("state"), Some(&"all".to_string()));
}

#[test]
fn a_nested_collections_context_is_resolved_from_its_parent() {
    let model = discover_resource_model(&github_like_document()).expect("model derives");

    let comments = model
        .by_name("issueComments")
        .expect("issueComments collection");
    assert_eq!(comments.resource, "issueComment");
    // Comments are listed under the parent issue, but addressed individually
    // at a non-nested path — the collection and item URLs don't share a prefix.
    assert_eq!(
        comments.collection_url,
        "/repos/{owner}/{repo}/issues/{issue_number}/comments"
    );
    assert_eq!(
        comments.item_url,
        "/repos/{owner}/{repo}/issues/comments/{comment_id}"
    );
    assert_eq!(comments.id_field, "id");
    assert_eq!(
        comments.context_params,
        vec![
            "owner".to_string(),
            "repo".to_string(),
            "issue_number".to_string()
        ]
    );

    let provider = model
        .provider_for("issue_number")
        .expect("issue_number has a provider");
    assert_eq!(provider.collection, "issues");
    assert_eq!(provider.field, "number");
}

#[test]
fn a_constant_only_context_param_has_no_provider() {
    // `owner`/`repo` are supplied by configuration (issue #6), not by any
    // enumerable collection.
    let model = discover_resource_model(&github_like_document()).expect("model derives");
    assert!(model.provider_for("owner").is_none());
    assert!(model.provider_for("repo").is_none());
}

#[test]
fn reads_a_list_operations_x_crud() {
    let document = github_like_document();
    let operation = document.paths["/repos/{owner}/{repo}/issues"]
        .get
        .as_ref()
        .expect("get exists");
    let crud = crud_operation(operation)
        .expect("valid x-crud")
        .expect("x-crud present");
    assert_eq!(crud.action, CrudAction::List);
    assert_eq!(crud.resource, "issue");
    assert_eq!(crud.collection.as_deref(), Some("issues"));
}

#[test]
fn reads_a_create_operations_added_fields_and_membership() {
    let document = github_like_document();
    let operation = document.paths["/repos/{owner}/{repo}/issues"]
        .post
        .as_ref()
        .expect("post exists");
    let crud = crud_operation(operation)
        .expect("valid x-crud")
        .expect("x-crud present");
    assert_eq!(crud.action, CrudAction::Create);
    assert_eq!(crud.member_of, vec!["issues".to_string()]);

    let number = crud
        .added_fields
        .get("number")
        .expect("number is server-assigned");
    assert_eq!(number.source.as_deref(), Some("server"));
    assert_eq!(
        number
            .schema
            .as_ref()
            .and_then(|s| s.schema_type.as_ref())
            .and_then(SchemaType::primary),
        Some("integer")
    );
}

#[test]
fn reads_an_update_operations_mode_and_patch_format() {
    let document = github_like_document();
    let operation = document.paths["/repos/{owner}/{repo}/issues/{issue_number}"]
        .patch
        .as_ref()
        .expect("patch exists");
    let crud = crud_operation(operation)
        .expect("valid x-crud")
        .expect("x-crud present");
    assert_eq!(crud.action, CrudAction::Update);
    assert_eq!(crud.mode.as_deref(), Some("patch"));
    assert_eq!(crud.patch_format.as_deref(), Some("merge"));
}

#[test]
fn reads_a_delete_operations_wildcard_removes_from() {
    let document = github_like_document();
    let operation = document.paths["/repos/{owner}/{repo}/issues/comments/{comment_id}"]
        .delete
        .as_ref()
        .expect("delete exists");
    let crud = crud_operation(operation)
        .expect("valid x-crud")
        .expect("x-crud present");
    assert_eq!(crud.action, CrudAction::Delete);
    assert_eq!(crud.removes_from, Some(CollectionMembership::All));
}

#[test]
fn removes_from_also_accepts_a_named_collection_list() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "paths": {
            "/widgets/{id}": {
                "delete": {
                    "x-crud": {
                        "action": "delete",
                        "resource": "widget",
                        "removesFrom": ["widgets", "archivedWidgets"]
                    }
                }
            }
        }
    }))
    .expect("valid document");
    let operation = document.paths["/widgets/{id}"]
        .delete
        .as_ref()
        .expect("delete exists");
    let crud = crud_operation(operation)
        .expect("valid x-crud")
        .expect("x-crud present");
    assert_eq!(
        crud.removes_from,
        Some(CollectionMembership::Named(vec![
            "widgets".to_string(),
            "archivedWidgets".to_string()
        ]))
    );
}

#[test]
fn an_operation_without_x_crud_has_none() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "paths": { "/healthz": { "get": {} } }
    }))
    .expect("valid document");
    let operation = document.paths["/healthz"].get.as_ref().expect("get exists");
    assert!(crud_operation(operation)
        .expect("no x-crud is not an error")
        .is_none());
}

#[test]
fn a_malformed_removes_from_is_a_json_error() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "paths": {
            "/widgets/{id}": {
                "delete": {
                    "x-crud": { "action": "delete", "resource": "widget", "removesFrom": 42 }
                }
            }
        }
    }))
    .expect("valid document");
    let operation = document.paths["/widgets/{id}"]
        .delete
        .as_ref()
        .expect("delete exists");
    let error = crud_operation(operation).expect_err("removesFrom must be \"*\" or an array");
    assert!(matches!(error, Error::Json(_)));
}
