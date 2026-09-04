//! `SyncClient::sync()` — the read-only half of #9 — exercised against a
//! [`MockFetch`] test double rather than a live server, the same way the
//! rest of this crate's pagination/resource-model machinery is tested.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use syncables::client::client::{Fetch, HttpRequest, HttpResponse};
use syncables::{ClientConfig, Credentials, InMemoryStorage, Storage, SyncClient, SyncError};

/// One canned response a [`MockFetch`] serves for an exact URL.
struct MockResponse {
    status: u16,
    headers: IndexMap<String, String>,
    body: Vec<u8>,
}

/// A [`Fetch`] test double: serves a fixed response per exact URL, and
/// records every URL it was asked for. A URL with no registered response
/// errors rather than panicking, so a test can assert on the resulting
/// `SyncReport::errors` instead.
#[derive(Default)]
struct MockFetch {
    responses: HashMap<String, MockResponse>,
    requested: Mutex<Vec<String>>,
}

impl MockFetch {
    fn respond_json(
        mut self,
        url: impl Into<String>,
        status: u16,
        headers: &[(&str, &str)],
        body: Value,
    ) -> Self {
        self.responses.insert(
            url.into(),
            MockResponse {
                status,
                headers: headers
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
                body: serde_json::to_vec(&body).expect("serializable"),
            },
        );
        self
    }

    fn requested_urls(&self) -> Vec<String> {
        self.requested.lock().expect("mutex poisoned").clone()
    }
}

#[async_trait]
impl Fetch for MockFetch {
    async fn fetch(&self, request: HttpRequest) -> syncables::Result<HttpResponse> {
        self.requested
            .lock()
            .expect("mutex poisoned")
            .push(request.url.clone());
        match self.responses.get(&request.url) {
            Some(response) => Ok(HttpResponse {
                status: response.status,
                headers: response.headers.clone(),
                body: response.body.clone(),
            }),
            None => Err(syncables::Error::Http(format!(
                "no mock response registered for {}",
                request.url
            ))),
        }
    }
}

fn constants(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Writes `document` to a uniquely-named file in the OS temp directory and
/// returns its path — `SyncClient` loads a document from a `PathBuf`, not
/// an in-memory value, so tests need a real file on disk.
fn write_document(document: &Value) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "syncables-sync-client-test-{}.json",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, serde_json::to_vec(document).expect("serializable"))
        .expect("write fixture document");
    path
}

fn owner_repo_parameters() -> Value {
    json!([
        { "name": "owner", "in": "path", "required": true, "schema": { "type": "string" } },
        { "name": "repo", "in": "path", "required": true, "schema": { "type": "string" } }
    ])
}

fn array_response() -> Value {
    json!({
        "200": {
            "content": { "application/json": { "schema": { "type": "array", "items": { "type": "object" } } } }
        }
    })
}

/// A document with a single, non-nested `issue` resource — no pagination.
fn flat_document(servers: Option<Value>) -> Value {
    let mut document = json!({
        "openapi": "3.0.3",
        "info": { "title": "Widgets", "version": "1.0.0" },
        "paths": {
            "/repos/{owner}/{repo}/issues": {
                "get": {
                    "parameters": owner_repo_parameters(),
                    "responses": array_response()
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
                }
            }
        }
    });
    if let Some(servers) = servers {
        document["servers"] = servers;
    }
    document
}

/// A document with `issue` and a nested `issueComment` resource — walked
/// once per `issue` record.
fn nested_document() -> Value {
    let mut document = flat_document(Some(json!([{ "url": "https://api.example.com" }])));
    document["paths"]["/repos/{owner}/{repo}/issues/{issue_number}/comments"] = json!({
        "get": {
            "parameters": [
                { "name": "owner", "in": "path", "required": true, "schema": { "type": "string" } },
                { "name": "repo", "in": "path", "required": true, "schema": { "type": "string" } },
                { "name": "issue_number", "in": "path", "required": true, "schema": { "type": "integer" } }
            ],
            "responses": array_response()
        }
    });
    document["components"]["crudResources"]["issueComment"] = json!({
        "identity": {
            "urlTemplate": "/repos/{owner}/{repo}/issues/comments/{comment_id}",
            "bindings": { "comment_id": { "field": "id" } }
        },
        "collections": {
            "issueComments": { "urlTemplate": "/repos/{owner}/{repo}/issues/{issue_number}/comments" }
        }
    });
    document
}

fn config(document: &Path, fetch_constants: &[(&str, &str)]) -> ClientConfig {
    ClientConfig {
        document: document.to_path_buf(),
        overlays: Vec::new(),
        credentials: Credentials::Anonymous,
        constants: constants(fetch_constants),
        ontology_base_url: "https://my-ontologies.com".to_string(),
    }
}

#[tokio::test]
async fn syncs_a_flat_collection_into_storage() {
    let document = write_document(&flat_document(Some(
        json!([{ "url": "https://api.example.com" }]),
    )));
    let fetch = MockFetch::default().respond_json(
        "https://api.example.com/repos/acme/widgets/issues",
        200,
        &[],
        json!([
            { "number": 1, "title": "First issue" },
            { "number": 2, "title": "Second issue" }
        ]),
    );
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let report = client.sync(&storage).await.expect("sync succeeds");

    assert!(
        report.errors.is_empty(),
        "unexpected errors: {:?}",
        report.errors
    );
    assert_eq!(report.read.get("issue"), Some(&2));
    assert_eq!(report.ontology_terms, 1);
    assert_eq!(storage.ontologies().len(), 1);

    let first = storage
        .get("acme/widgets", "issue", "1")
        .await
        .expect("get succeeds")
        .expect("record present");
    assert_eq!(first.value.get("title"), Some(&json!("First issue")));
}

#[tokio::test]
async fn walks_a_next_link_paginated_collection_across_pages() {
    let mut document = flat_document(Some(json!([{ "url": "https://api.example.com" }])));
    document["paths"]["/repos/{owner}/{repo}/issues"]["get"]["x-pagination"] =
        json!([{ "scheme": "nextLink" }]);
    document["components"]["paginationSchemes"] = json!({
        "nextLink": {
            "type": "nextLink",
            "response": { "headers": { "Link": { "role": "nextLink" } } }
        }
    });
    let document = write_document(&document);

    let fetch = MockFetch::default()
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues",
            200,
            &[(
                "Link",
                "<https://api.example.com/repos/acme/widgets/issues?page=2>; rel=\"next\"",
            )],
            json!([{ "number": 1, "title": "First issue" }]),
        )
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues?page=2",
            200,
            &[],
            json!([{ "number": 2, "title": "Second issue" }]),
        );
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let report = client.sync(&storage).await.expect("sync succeeds");

    assert!(
        report.errors.is_empty(),
        "unexpected errors: {:?}",
        report.errors
    );
    assert_eq!(report.read.get("issue"), Some(&2));
    for id in ["1", "2"] {
        assert!(storage
            .get("acme/widgets", "issue", id)
            .await
            .expect("get succeeds")
            .is_some());
    }
}

#[tokio::test]
async fn walks_a_nested_collection_once_per_parent_record() {
    let document = write_document(&nested_document());
    let fetch = MockFetch::default()
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues",
            200,
            &[],
            json!([{ "number": 1, "title": "First" }, { "number": 2, "title": "Second" }]),
        )
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues/1/comments",
            200,
            &[],
            json!([{ "id": 100, "body": "hi" }]),
        )
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues/2/comments",
            200,
            &[],
            json!([{ "id": 200, "body": "yo" }]),
        );
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let report = client.sync(&storage).await.expect("sync succeeds");

    assert!(
        report.errors.is_empty(),
        "unexpected errors: {:?}",
        report.errors
    );
    assert_eq!(report.read.get("issue"), Some(&2));
    assert_eq!(report.read.get("issueComment"), Some(&2));

    // Each issue's comments are namespaced under that issue, not lumped
    // together — otherwise issue 2's comment would overwrite issue 1's.
    let comment_1 = storage
        .get("acme/widgets/1", "issueComment", "100")
        .await
        .expect("get succeeds")
        .expect("record present");
    assert_eq!(comment_1.value.get("body"), Some(&json!("hi")));
    let comment_2 = storage
        .get("acme/widgets/2", "issueComment", "200")
        .await
        .expect("get succeeds")
        .expect("record present");
    assert_eq!(comment_2.value.get("body"), Some(&json!("yo")));
}

#[tokio::test]
async fn a_failing_nested_collection_is_a_partial_failure_not_a_fatal_one() {
    let document = write_document(&nested_document());
    let fetch = MockFetch::default()
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues",
            200,
            &[],
            json!([{ "number": 1, "title": "First" }]),
        )
        .respond_json(
            "https://api.example.com/repos/acme/widgets/issues/1/comments",
            404,
            &[],
            json!({}),
        );
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let report = client
        .sync(&storage)
        .await
        .expect("sync still succeeds overall");

    // The issue itself synced fine...
    assert_eq!(report.read.get("issue"), Some(&1));
    assert!(storage
        .get("acme/widgets", "issue", "1")
        .await
        .expect("get succeeds")
        .is_some());
    // ...and the failing nested collection is recorded, not fatal.
    assert_eq!(report.errors.len(), 1);
    assert!(
        report.errors[0].contains("404"),
        "error was: {}",
        report.errors[0]
    );
}

#[tokio::test]
async fn reports_the_bound_url_when_a_collection_request_fails() {
    let document = write_document(&flat_document(Some(
        json!([{ "url": "https://api.example.com" }]),
    )));
    // Registering a response only for the concrete URL also verifies that
    // configured path constants are substituted before Fetch is called.
    let fetch = MockFetch::default().respond_json(
        "https://api.example.com/repos/localthought/test-repo-1/issues",
        403,
        &[],
        json!({}),
    );
    let client = SyncClient::new(
        config(
            &document,
            &[("owner", "localthought"), ("repo", "test-repo-1")],
        ),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let report = client
        .sync(&storage)
        .await
        .expect("a collection failure is reported, not fatal");

    assert_eq!(
        report.errors,
        [
            "issues: GET https://api.example.com/repos/localthought/test-repo-1/issues responded 403"
                .to_string()
        ]
    );
}

#[tokio::test]
async fn a_storage_failure_is_a_partial_failure_not_a_fatal_one() {
    struct RejectsIssueOne(InMemoryStorage);

    #[async_trait]
    impl Storage for RejectsIssueOne {
        async fn put(&self, record: &syncables::Record) -> Result<(), syncables::StorageError> {
            if record.id == "1" {
                return Err(syncables::StorageError::new("rejected"));
            }
            self.0.put(record).await
        }
        async fn get(
            &self,
            namespace: &str,
            resource: &str,
            id: &str,
        ) -> Result<Option<syncables::Record>, syncables::StorageError> {
            self.0.get(namespace, resource, id).await
        }
        async fn list(
            &self,
            namespace: &str,
            resource: &str,
        ) -> Result<Vec<syncables::Record>, syncables::StorageError> {
            self.0.list(namespace, resource).await
        }
        async fn delete(
            &self,
            namespace: &str,
            resource: &str,
            id: &str,
        ) -> Result<(), syncables::StorageError> {
            self.0.delete(namespace, resource, id).await
        }
        async fn put_ontology(
            &self,
            ontology: &syncables::Ontology,
        ) -> Result<(), syncables::StorageError> {
            self.0.put_ontology(ontology).await
        }
    }

    let document = write_document(&flat_document(Some(
        json!([{ "url": "https://api.example.com" }]),
    )));
    let fetch = MockFetch::default().respond_json(
        "https://api.example.com/repos/acme/widgets/issues",
        200,
        &[],
        json!([{ "number": 1, "title": "First" }, { "number": 2, "title": "Second" }]),
    );
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(fetch),
    )
    .expect("valid config");

    let storage = RejectsIssueOne(InMemoryStorage::new());
    let report = client
        .sync(&storage)
        .await
        .expect("sync still succeeds overall");

    assert_eq!(report.errors.len(), 1);
    assert!(
        report.errors[0].contains("rejected"),
        "error was: {}",
        report.errors[0]
    );
    assert!(storage
        .0
        .get("acme/widgets", "issue", "2")
        .await
        .expect("get succeeds")
        .is_some());
    assert!(storage
        .0
        .get("acme/widgets", "issue", "1")
        .await
        .expect("get succeeds")
        .is_none());
}

#[tokio::test]
async fn new_rejects_an_empty_ontology_base_url() {
    let config = ClientConfig {
        document: PathBuf::from("/does/not/matter"),
        overlays: Vec::new(),
        credentials: Credentials::Anonymous,
        constants: BTreeMap::new(),
        ontology_base_url: String::new(),
    };
    let error =
        SyncClient::new(config, Arc::new(MockFetch::default())).expect_err("empty base url");
    assert!(matches!(error, SyncError::Document(_)));
}

#[tokio::test]
async fn sync_errors_when_the_document_declares_no_servers() {
    let document = write_document(&flat_document(None));
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        Arc::new(MockFetch::default()),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let error = client
        .sync(&storage)
        .await
        .expect_err("no servers declared");
    assert!(matches!(error, SyncError::Document(message) if message.contains("servers")));
    // Nothing should have been written — the sync fails before any writes.
    assert!(storage.ontologies().is_empty());
}

#[tokio::test]
async fn sync_errors_when_a_constant_is_unresolvable() {
    // `repo` is never supplied, and the document declares no way to
    // enumerate it either.
    let document = write_document(&flat_document(Some(
        json!([{ "url": "https://api.example.com" }]),
    )));
    let client = SyncClient::new(
        config(&document, &[("owner", "acme")]),
        Arc::new(MockFetch::default()),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    let error = client.sync(&storage).await.expect_err("repo is unbound");
    assert!(matches!(error, SyncError::Document(_)));
}

#[tokio::test]
async fn tracks_every_url_it_requested() {
    let document = write_document(&flat_document(Some(
        json!([{ "url": "https://api.example.com" }]),
    )));
    let fetch = Arc::new(MockFetch::default().respond_json(
        "https://api.example.com/repos/acme/widgets/issues",
        200,
        &[],
        json!([]),
    ));
    let client = SyncClient::new(
        config(&document, &[("owner", "acme"), ("repo", "widgets")]),
        fetch.clone(),
    )
    .expect("valid config");

    let storage = InMemoryStorage::new();
    client.sync(&storage).await.expect("sync succeeds");

    assert_eq!(
        fetch.requested_urls(),
        vec!["https://api.example.com/repos/acme/widgets/issues".to_string()]
    );
}
