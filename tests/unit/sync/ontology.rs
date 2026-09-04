use serde_json::json;
use syncables::{derive_ontology, Error, OntologyTerm, OpenApiDocument, TermKind};

/// The GitHub crud-causality overlay's own example, with the `schema` refs
/// it declares resolved against `components.schemas` — same shape a real
/// `load_open_api_document_with_overlays` call would produce.
fn github_like_document() -> OpenApiDocument {
    serde_json::from_value(json!({
        "info": { "title": "GitHub Issues", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "issue": {
                    "type": "object",
                    "description": "An issue in a repository.",
                    "required": ["title"],
                    "properties": {
                        "title": { "type": "string", "description": "The issue's title." },
                        "number": { "type": "integer", "description": "Per-repository issue number." },
                        "state_reason": { "type": "string" },
                        "updated_at": { "type": "string", "format": "date-time" },
                        "body": { "type": "string" }
                    }
                },
                "issue-comment": {
                    "type": "object",
                    "required": ["body"],
                    "properties": {
                        "id": { "type": "integer" },
                        "body": { "type": "string" },
                        "updated_at": { "type": "string", "format": "date-time" }
                    }
                }
            },
            "crudResources": {
                "issue": {
                    "description": "An issue in a repository.",
                    "schema": { "$ref": "#/components/schemas/issue" },
                    "identity": {
                        "urlTemplate": "/repos/{owner}/{repo}/issues/{issue_number}",
                        "bindings": { "issue_number": { "field": "number" } }
                    },
                    "collections": {
                        "issues": { "urlTemplate": "/repos/{owner}/{repo}/issues" }
                    }
                },
                "issueComment": {
                    "schema": { "$ref": "#/components/schemas/issue-comment" },
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

fn term<'a>(ontology: &'a syncables::Ontology, path: &str) -> &'a OntologyTerm {
    ontology
        .terms
        .iter()
        .find(|t| t.path == path)
        .unwrap_or_else(|| panic!("no term at {path}"))
}

#[test]
fn errors_when_the_document_declares_no_crud_resources() {
    let document = OpenApiDocument::default();
    let error = derive_ontology(&document).expect_err("no crudResources declared");
    assert!(matches!(error, Error::NoCrudResources));
}

#[test]
fn derives_a_class_per_resource() {
    let ontology = derive_ontology(&github_like_document()).expect("ontology derives");
    assert_eq!(ontology.path, "github-issues");
    assert_eq!(ontology.shortname, "github-issues");

    let issue = term(&ontology, "github-issues/class/issue");
    assert_eq!(issue.kind, TermKind::Class);
    assert_eq!(issue.shortname, "issue");
    assert_eq!(issue.description, "An issue in a repository.");
}

#[test]
fn a_field_normalizes_its_snake_case_shortname() {
    let ontology = derive_ontology(&github_like_document()).expect("ontology derives");
    // `state_reason` -> `state-reason`, and the datatype mapping omits a
    // type it can't place (there is none here, but the shortname is the
    // point of this test).
    let state_reason = term(&ontology, "github-issues/property/state-reason");
    assert_eq!(state_reason.shortname, "state-reason");
}

#[test]
fn maps_known_schema_types_to_atomic_data_datatypes() {
    let ontology = derive_ontology(&github_like_document()).expect("ontology derives");
    assert_eq!(
        term(&ontology, "github-issues/property/title")
            .datatype
            .as_deref(),
        Some("https://atomicdata.dev/datatypes/string")
    );
    assert_eq!(
        term(&ontology, "github-issues/property/number")
            .datatype
            .as_deref(),
        Some("https://atomicdata.dev/datatypes/integer")
    );
    assert_eq!(
        term(&ontology, "github-issues/property/updated-at")
            .datatype
            .as_deref(),
        Some("https://atomicdata.dev/datatypes/timestamp")
    );
}

#[test]
fn a_required_field_lands_in_requires_not_recommends() {
    let ontology = derive_ontology(&github_like_document()).expect("ontology derives");
    let issue = term(&ontology, "github-issues/class/issue");
    assert!(issue
        .requires
        .contains(&"github-issues/property/title".to_string()));
    assert!(!issue
        .recommends
        .contains(&"github-issues/property/title".to_string()));
    assert!(issue
        .recommends
        .contains(&"github-issues/property/number".to_string()));
}

#[test]
fn a_field_shared_across_resources_is_one_property_not_two() {
    let ontology = derive_ontology(&github_like_document()).expect("ontology derives");
    // Both `issue` and `issueComment` schemas declare `body`/`updated_at`.
    let body_terms = ontology
        .terms
        .iter()
        .filter(|t| t.path == "github-issues/property/body")
        .count();
    assert_eq!(
        body_terms, 1,
        "body must be minted once, not once per resource"
    );

    let issue = term(&ontology, "github-issues/class/issue");
    // The class shortname is slugified too: "issueComment" -> "issuecomment"
    // (camelCase casing is folded, not dash-separated — only runs of
    // non-alphanumeric characters become a `-`).
    let comment = term(&ontology, "github-issues/class/issuecomment");
    assert!(issue
        .recommends
        .contains(&"github-issues/property/body".to_string()));
    assert!(comment
        .requires
        .contains(&"github-issues/property/body".to_string()));
}

#[test]
fn a_genuine_shortname_collision_is_an_error() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "info": { "title": "Widgets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "widget": {
                    "type": "object",
                    "properties": {
                        "state_reason": { "type": "string" },
                        "state-reason": { "type": "string" }
                    }
                }
            },
            "crudResources": {
                "widget": {
                    "schema": { "$ref": "#/components/schemas/widget" },
                    "identity": { "urlTemplate": "/widgets/{id}" },
                    "collections": { "widgets": { "urlTemplate": "/widgets" } }
                }
            }
        }
    }))
    .expect("valid document");

    let error = derive_ontology(&document)
        .expect_err("state_reason and state-reason collide on the same shortname");
    assert!(
        matches!(error, Error::ShortnameCollision { shortname, .. } if shortname == "state-reason")
    );
}

#[test]
fn an_unmapped_schema_type_omits_the_datatype() {
    let document: OpenApiDocument = serde_json::from_value(json!({
        "info": { "title": "Widgets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "widget": {
                    "type": "object",
                    "properties": { "tags": { "type": "array", "items": { "type": "string" } } }
                }
            },
            "crudResources": {
                "widget": {
                    "schema": { "$ref": "#/components/schemas/widget" },
                    "identity": { "urlTemplate": "/widgets/{id}" },
                    "collections": { "widgets": { "urlTemplate": "/widgets" } }
                }
            }
        }
    }))
    .expect("valid document");

    let ontology = derive_ontology(&document).expect("ontology derives");
    assert_eq!(term(&ontology, "widgets/property/tags").datatype, None);
}
