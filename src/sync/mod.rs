//! The sync engine surface: `SyncClient`, `ClientConfig`, [`credentials`]
//! and friends.
//!
//! This is a *different* surface from the crate's existing
//! [`crate::client`]/[`crate::mock_server`] pair (a port of the
//! TypeScript `syncables` package). It is new scope, tracked by
//! [localthought/syncables-rs#1](https://github.com/localthought/syncables-rs/issues/1)
//! and the issues under it: a generic engine that reads an OpenAPI
//! document plus a resource model derived from it, and syncs records into
//! a host-provided `Storage` implementation.
//! [`localthought/reflector-rs`](https://github.com/localthought/reflector-rs)
//! is the first intended host.
//!
//! Only [`credentials`] exists so far (issue #5). The rest of this module —
//! `SyncClient`, `ClientConfig`, `SyncError`, `SyncReport`, the resource
//! model, the `Storage` trait — lands with the other issues in that series.

pub mod credentials;
