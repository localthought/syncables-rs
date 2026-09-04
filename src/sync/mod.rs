//! The sync engine surface: `SyncClient`, `ClientConfig`, [`storage::Storage`]
//! and friends.
//!
//! This is a *different* surface from the crate's existing
//! [`crate::client`]/[`crate::mock_server`] pair (a port of the
//! TypeScript `syncables` package). It is new scope, tracked by
//! [localthought/syncables-rs#1](https://github.com/localthought/syncables-rs/issues/1)
//! and the issues under it: a generic engine that reads an OpenAPI
//! document plus a resource model derived from it, and syncs records into
//! a host-provided [`storage::Storage`] implementation.
//! [`localthought/reflector-rs`](https://github.com/localthought/reflector-rs)
//! is the first intended host.
//!
//! Only [`storage`] exists so far (issue #7). The rest of this module —
//! `SyncClient`, `ClientConfig`, `SyncError`, `SyncReport`, `Credentials`,
//! the resource model — lands with the other issues in that series.

pub mod storage;
