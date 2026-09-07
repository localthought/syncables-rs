//! Integration tests, mirroring the TypeScript original's `__tests__/unit/`
//! layout one-to-one.
//!
//! Cargo picks up `tests/unit/main.rs` as one test binary, so the tree
//! below is declared here rather than discovered per file — the file paths
//! still match `__tests__/unit/**` module for module.

// Test helpers take `serde_json::Value` by value for call-site readability.
#![allow(clippy::needless_pass_by_value)]
// The ported fixtures are single long JSON literals.
#![allow(clippy::too_many_lines)]

#[path = "../fixtures/pets.rs"]
pub mod pets;

mod acceptance {
    mod real_world;
}

mod client {
    mod storage;
}

mod fake_data {
    mod generate;
}

mod mock_server {
    mod store;
}

mod openapi {
    mod overlay;
    mod resolve_refs;
    mod types;
}

mod pagination {
    mod autodetect;
    mod items;
    mod request_builder;
    mod response_parser;
    mod validate;
}

mod resources {
    mod discover;
}

mod routing {
    mod router;
}

mod sync {
    mod client;
    mod constants;
    mod credentials;
    mod ontology;
    mod resource_model;
    mod storage;
}
