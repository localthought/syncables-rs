use serde_json::{json, Value};
use syncables::{InMemoryStorage, Ontology, Record, Storage, StorageError};

fn record(namespace: &str, resource: &str, id: &str, value: Value) -> Record {
    Record {
        namespace: namespace.to_string(),
        resource: resource.to_string(),
        id: id.to_string(),
        value: value.as_object().cloned().expect("object"),
    }
}

fn ontology() -> Ontology {
    Ontology {
        path: "github-issues".to_string(),
        shortname: "github-issues".to_string(),
        description: "Derived from the GitHub Issues OpenAPI document.".to_string(),
        terms: Vec::new(),
    }
}

fn value(name: &str) -> Value {
    json!({ "name": name })
}

#[tokio::test]
async fn round_trips_a_record() {
    let storage = InMemoryStorage::new();
    let milo = record("cal-1", "event", "1", value("Milo"));
    storage.put(&milo).await.expect("put succeeds");

    assert_eq!(
        storage
            .get("cal-1", "event", "1")
            .await
            .expect("get succeeds"),
        Some(milo.clone())
    );
    assert_eq!(
        storage.list("cal-1", "event").await.expect("list succeeds"),
        vec![milo]
    );
}

#[tokio::test]
async fn put_replaces_an_existing_record() {
    let storage = InMemoryStorage::new();
    storage
        .put(&record("cal-1", "event", "1", value("Milo")))
        .await
        .expect("put");
    storage
        .put(&record("cal-1", "event", "1", value("Rex")))
        .await
        .expect("put");

    assert_eq!(
        storage.get("cal-1", "event", "1").await.expect("get"),
        Some(record("cal-1", "event", "1", value("Rex")))
    );
    assert_eq!(storage.list("cal-1", "event").await.expect("list").len(), 1);
}

#[tokio::test]
async fn lists_an_unknown_namespace_or_resource_as_empty() {
    let storage = InMemoryStorage::new();
    assert!(storage
        .list("nope", "event")
        .await
        .expect("list succeeds")
        .is_empty());
    assert!(storage
        .get("nope", "event", "1")
        .await
        .expect("get succeeds")
        .is_none());
}

#[tokio::test]
async fn deletes_a_record() {
    let storage = InMemoryStorage::new();
    storage
        .put(&record("cal-1", "event", "1", value("Milo")))
        .await
        .expect("put");
    storage.delete("cal-1", "event", "1").await.expect("delete");
    assert!(storage
        .get("cal-1", "event", "1")
        .await
        .expect("get")
        .is_none());
}

#[tokio::test]
async fn deleting_an_absent_record_is_not_an_error() {
    let storage = InMemoryStorage::new();
    storage
        .delete("cal-1", "event", "nope")
        .await
        .expect("delete");
}

#[tokio::test]
async fn namespaces_isolate_identical_resource_id_pairs() {
    let storage = InMemoryStorage::new();
    storage
        .put(&record(
            "issue-1",
            "issueComment",
            "c1",
            value("first issue's comment"),
        ))
        .await
        .expect("put");
    storage
        .put(&record(
            "issue-2",
            "issueComment",
            "c1",
            value("second issue's comment"),
        ))
        .await
        .expect("put");

    assert_eq!(
        storage
            .get("issue-1", "issueComment", "c1")
            .await
            .expect("get"),
        Some(record(
            "issue-1",
            "issueComment",
            "c1",
            value("first issue's comment")
        ))
    );
    assert_eq!(
        storage
            .get("issue-2", "issueComment", "c1")
            .await
            .expect("get"),
        Some(record(
            "issue-2",
            "issueComment",
            "c1",
            value("second issue's comment")
        ))
    );
    assert_eq!(
        storage
            .list("issue-1", "issueComment")
            .await
            .expect("list")
            .len(),
        1
    );
    assert_eq!(
        storage
            .list("issue-2", "issueComment")
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
async fn list_only_returns_the_requested_namespace_and_resource() {
    let storage = InMemoryStorage::new();
    storage
        .put(&record("issue-1", "issue", "1", value("issue one")))
        .await
        .expect("put");
    storage
        .put(&record("issue-1", "issueComment", "c1", value("a comment")))
        .await
        .expect("put");
    storage
        .put(&record(
            "issue-2",
            "issueComment",
            "c2",
            value("another issue's comment"),
        ))
        .await
        .expect("put");

    let comments = storage.list("issue-1", "issueComment").await.expect("list");
    assert_eq!(
        comments,
        vec![record("issue-1", "issueComment", "c1", value("a comment"))]
    );
}

#[tokio::test]
async fn put_ontology_is_accepted_and_recorded() {
    let storage = InMemoryStorage::new();
    assert!(storage.ontologies().is_empty());

    storage
        .put_ontology(&ontology())
        .await
        .expect("put_ontology succeeds");

    assert_eq!(storage.ontologies(), vec![ontology()]);
}

#[tokio::test]
async fn is_usable_as_a_trait_object() {
    let storage: std::sync::Arc<dyn Storage> = std::sync::Arc::new(InMemoryStorage::new());
    storage
        .put(&record("cal-1", "event", "1", value("Milo")))
        .await
        .expect("put");
    storage
        .put_ontology(&ontology())
        .await
        .expect("put_ontology");
    assert_eq!(storage.list("cal-1", "event").await.expect("list").len(), 1);
}

/// A [`Storage`] that always fails, to exercise `StorageError` propagation.
struct FailingStorage;

#[async_trait::async_trait]
impl Storage for FailingStorage {
    async fn put(&self, _record: &Record) -> Result<(), StorageError> {
        Err(StorageError::new("backend unavailable"))
    }

    async fn get(
        &self,
        _namespace: &str,
        _resource: &str,
        _id: &str,
    ) -> Result<Option<Record>, StorageError> {
        Err(StorageError::new("backend unavailable"))
    }

    async fn list(&self, _namespace: &str, _resource: &str) -> Result<Vec<Record>, StorageError> {
        Err(StorageError::new("backend unavailable"))
    }

    async fn delete(
        &self,
        _namespace: &str,
        _resource: &str,
        _id: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::with_source(
            "backend unavailable",
            std::io::Error::other("disk gone"),
        ))
    }

    async fn put_ontology(&self, _ontology: &Ontology) -> Result<(), StorageError> {
        Err(StorageError::new("backend unavailable"))
    }
}

#[tokio::test]
async fn storage_errors_propagate_with_their_message() {
    let storage = FailingStorage;

    let err = storage
        .put(&record("cal-1", "event", "1", value("Milo")))
        .await
        .unwrap_err();
    assert_eq!(err.message, "backend unavailable");
    assert_eq!(err.to_string(), "backend unavailable");
    assert!(err.source.is_none());

    let err = storage.get("cal-1", "event", "1").await.unwrap_err();
    assert_eq!(err.message, "backend unavailable");

    let err = storage.list("cal-1", "event").await.unwrap_err();
    assert_eq!(err.message, "backend unavailable");

    let err = storage.put_ontology(&ontology()).await.unwrap_err();
    assert_eq!(err.message, "backend unavailable");
}

#[tokio::test]
async fn storage_error_carries_its_source() {
    use std::error::Error as _;

    let storage = FailingStorage;
    let err = storage.delete("cal-1", "event", "1").await.unwrap_err();
    assert_eq!(err.message, "backend unavailable");
    let source = err.source().expect("has a source");
    assert_eq!(source.to_string(), "disk gone");
}
