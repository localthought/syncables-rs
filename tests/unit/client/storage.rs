use serde_json::{json, Map, Value};
use syncables::{InMemoryStorageAdapter, StorageAdapter};

fn record(name: &str) -> Map<String, Value> {
    json!({ "name": name })
        .as_object()
        .cloned()
        .expect("object")
}

#[tokio::test]
async fn round_trips_a_record() {
    let storage = InMemoryStorageAdapter::new();
    storage
        .put("/pets", "1", record("Milo"))
        .await
        .expect("put succeeds");

    assert_eq!(
        storage.get("/pets", "1").await.expect("get succeeds"),
        Some(record("Milo"))
    );
    assert_eq!(
        storage.list("/pets").await.expect("list succeeds"),
        vec![record("Milo")]
    );
}

#[tokio::test]
async fn lists_an_unknown_resource_as_empty() {
    let storage = InMemoryStorageAdapter::new();
    assert!(storage
        .list("/nope")
        .await
        .expect("list succeeds")
        .is_empty());
    assert!(storage
        .get("/nope", "1")
        .await
        .expect("get succeeds")
        .is_none());
}

#[tokio::test]
async fn deletes_a_record() {
    let storage = InMemoryStorageAdapter::new();
    storage
        .put("/pets", "1", record("Milo"))
        .await
        .expect("put");
    storage.delete("/pets", "1").await.expect("delete");
    assert!(storage.get("/pets", "1").await.expect("get").is_none());
}

#[tokio::test]
async fn preserves_insertion_order() {
    let storage = InMemoryStorageAdapter::new();
    storage
        .put("/pets", "1", record("Milo"))
        .await
        .expect("put");
    storage.put("/pets", "2", record("Rex")).await.expect("put");
    assert_eq!(
        storage.list("/pets").await.expect("list"),
        vec![record("Milo"), record("Rex")]
    );
}

#[tokio::test]
async fn is_usable_as_a_trait_object() {
    let storage: std::sync::Arc<dyn StorageAdapter> =
        std::sync::Arc::new(InMemoryStorageAdapter::new());
    storage
        .put("/pets", "1", record("Milo"))
        .await
        .expect("put");
    assert_eq!(storage.list("/pets").await.expect("list").len(), 1);
}
