use serde_json::{json, Map, Value};
use syncables::mock_server::store::ResourceStore;

fn record(name: &str) -> Map<String, Value> {
    json!({ "name": name })
        .as_object()
        .cloned()
        .expect("object")
}

#[test]
fn starts_empty_and_reports_no_collection() {
    let store = ResourceStore::new();
    assert!(!store.has("/pets"));
    assert!(store.etag("/pets").is_none());
    assert!(store.last_modified("/pets").is_none());
}

#[test]
fn round_trips_a_record() {
    let mut store = ResourceStore::new();
    store.put("/pets", "1", record("Milo"));

    assert_eq!(store.get("/pets", "1"), Some(record("Milo")));
    assert_eq!(store.list("/pets"), vec![record("Milo")]);
    assert!(store.has("/pets"));
}

#[test]
fn preserves_insertion_order() {
    let mut store = ResourceStore::new();
    store.put("/pets", "1", record("Milo"));
    store.put("/pets", "2", record("Rex"));
    assert_eq!(store.list("/pets"), vec![record("Milo"), record("Rex")]);
}

#[test]
fn deleting_reports_whether_the_record_existed() {
    let mut store = ResourceStore::new();
    store.put("/pets", "1", record("Milo"));
    assert!(store.delete("/pets", "1"));
    assert!(!store.delete("/pets", "1"));
    assert!(store.get("/pets", "1").is_none());
}

#[test]
fn the_etag_advances_on_every_mutation() {
    let mut store = ResourceStore::new();
    store.put("/pets", "1", record("Milo"));
    let first = store.etag("/pets").expect("populated");
    assert_eq!(first, "W/\"1\"");

    store.put("/pets", "2", record("Rex"));
    assert_eq!(store.etag("/pets").as_deref(), Some("W/\"2\""));

    // A delete that changed nothing must not bump the version.
    store.delete("/pets", "nope");
    assert_eq!(store.etag("/pets").as_deref(), Some("W/\"2\""));
    assert!(store.last_modified("/pets").is_some());
}

#[test]
fn collections_are_keyed_separately() {
    let mut store = ResourceStore::new();
    store.put("/pets", "1", record("Milo"));
    assert!(store.list("/toys").is_empty());
}
