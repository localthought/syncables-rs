# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`syncables-rs` is a Rust port of [localthought/syncables](https://github.com/localthought/syncables)
(TypeScript). It reads an OpenAPI document and produces two things from it:

- a **mock API server** (`create_mock_server`) that implements the
  document, backed by an in-memory CRUD store per resource, seeded with
  fake data generated from the document's schemas;
- an **API client** (`create_api_client`) that talks to any server
  implementing that document and keeps a local copy of each resource
  collection in sync.

Both understand the [OpenAPI Pagination Schemes Extension](https://github.com/pondersource/openapi-pagination-schemes-extension)
when a document declares `components.paginationSchemes`.

Alongside that port, the crate also carries a *second, unrelated* public
surface: a **sync engine** (`SyncClient`) that reads a document's
[CRUD Causality Extension](https://github.com/pondersource/openapi-extensions/tree/main/spec/crud-causality)
(`components.crudResources`) and syncs records — including nested
collections — into a host-provided `Storage` implementation, deriving an
Atomic-Data-shaped ontology along the way without depending on
`atomic_lib` itself. This is new scope, not part of the original
TypeScript port; see [The sync engine](#the-sync-engine-srcsync)
below and [issues #1–#9](https://github.com/localthought/syncables-rs/issues/1).

The public API surface is defined entirely by `src/lib.rs` re-exports
(mirroring the original's `src/index.ts` for the port, plus the sync
engine's own types) — check there first to see what's intended to be used
from outside the crate.

## Port status — read this before adding features

This section covers the *original TypeScript port* only — the sync
engine (`src/sync/`) is separate, untracked-by-this-table new scope; see
[its own section](#the-sync-engine-srcsync) for what exists there.

The port itself is **scaffolding**, not finished. `README.md` has the
per-module table. In short:

- Ported and tested: `openapi/`, `resources/`, `routing/`, `fake_data/`,
  all of `pagination/`, `mock_server/store.rs`, `client/storage.rs`.
- Scaffolded only: `mock_server/server.rs` (the HTTP request handler) and
  `client/client.rs` (sync, the local-first write queue, `paginate`).
  Their public surface, options structs, constants and doc comments are
  ported; the bodies are `todo!()`, each naming the TypeScript function it
  is to be ported from.

**When implementing one of those, port it — don't invent it.** Read the
corresponding file in the TypeScript original first and keep its
behaviour, its comments and its edge cases. The original's `CLAUDE.md`
documents the intended semantics in detail (conditional re-fetching, the
per-record write queue, id reconciliation, `MAX_PAGES`, seeding); that
document is the spec for this port.

## Commands

```sh
cargo build                                       # build the library
cargo test                                        # run the whole suite
cargo test --test unit pagination                 # one area
cargo test --test unit -- --exact routing::router::binds_path_variables
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all                                   # format in place
cargo fmt --all --check                           # check formatting
cargo build --target wasm32-unknown-unknown --lib # the library builds for wasm32; tests don't
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
```

CI (`.github/workflows/rust.yml`) runs fmt, clippy with `-D warnings`,
the test suite, a release build, and the two wasm32 checks above — run
those locally before pushing.

This crate has no binary target — it's a library, so there's no
`cargo run`.

**The library builds for `wasm32-unknown-unknown`** (see the README's
[WASM compatibility](README.md#wasm-compatibility) section for the two
adjustments that took — a `target.'cfg(...)'`-gated `tokio` dependency and
`uuid`'s `js` feature). Keep it that way: any new dependency or `tokio`
feature added to `[dependencies]` (not `[dev-dependencies]`, which never
builds for wasm32) needs checking against that target before it lands.

## Architecture

Data flows through four stages, each its own module under `src/`, matching
the original's directory layout one-to-one:

1. **`openapi/`** — `load.rs` reads a document from a file path or an
   in-memory `serde_json::Value` (YAML is a superset of JSON, so one
   parser covers both) and passes it through `resolve_refs.rs`, which
   inlines all local `#/...` JSON-pointer `$ref`s (with cycle/diamond-ref
   handling — see the comment in that file). Everything downstream assumes
   refs are already resolved; `types.rs` holds the minimal OpenAPI type
   surface actually used, each struct with a `#[serde(flatten)]`
   `extensions` catch-all so vendor extensions (notably `x-pagination`)
   survive a round-trip. `overlay.rs` is an intentionally minimal
   [Overlay](https://spec.openapis.org/overlay/v1.0.0.html)
   implementation: `update`/`remove` actions against `$`, dot-paths like
   `$.components`, and quoted bracket segments like
   `$.paths['/pets/{petId}'].get`, not the full JSONPath grammar (no
   wildcards, filters, or numeric/array indexing).
   `load_open_api_document_with_overlays` loads a document and applies a
   list of overlay files to it in order, after `$ref`s are resolved.
   Loading from a file path (`load_yaml_file`, via `tokio::fs`) is
   `#[cfg(not(target_arch = "wasm32"))]` only — that tokio feature has no
   wasm32 support — with a wasm32 stub that errors, directing callers to
   the in-memory `OpenApiSource::Value` path instead.

2. **`resources/discover.rs`** — turns `document.paths` into a list of
   `ResourceRoute`s by pairing each collection path (`/pets`) with its
   direct item-path child (`/pets/{petId}`). This pairing is the core
   concept the rest of the crate builds on: a "resource" only exists where
   that pairing holds. Paths without a matching item path (health checks,
   one-off actions) are not resources.

   Path parameters belonging to the *collection* itself (as opposed to the
   item id) aren't substituted anywhere downstream, and the mock server's
   `ResourceStore` keys collections by the raw path template, so a nested
   resource like GitHub's `/repos/{owner}/{repo}/issues` would collide
   every `{owner}`/`{repo}` combination into one shared collection. The
   current model only really supports resources at a fixed,
   parameter-free collection path. A sibling spec, the
   [OpenAPI CRUD Causality Extension](https://github.com/pondersource/openapi-extensions/tree/main/spec/crud-causality),
   formalizes a superset of this; it isn't implemented here either.

3. **`mock_server/`** — `store.rs` is the in-memory CRUD store (one
   `IndexMap` per collection path, plus a weak ETag and `Last-Modified`
   that advance on every mutation). `server.rs` is the request handler,
   still to be ported.

4. **`client/`** — `storage.rs` is the pluggable `StorageAdapter`
   (`InMemoryStorageAdapter` is the default). `client.rs` holds the
   client's public surface; sync, the local-first write queue and
   `paginate` are still to be ported.

`fake_data/generate.rs` (`generate_from_schema`) is the only place
schema-to-value synthesis lives — it prefers a schema's own `example`,
then `enum`, then handles `allOf`/`oneOf`/`anyOf`/type-based generation
recursively. A schema's fixed `example` is reused verbatim on every call,
so callers generating more than one item from the same schema must inject
their own unique `id` afterward.

### Pagination (`src/pagination/`)

- `types.rs` mirrors the extension's spec objects verbatim. Roles stay
  `String` rather than closed enums (the spec allows `x-` extension
  roles), and `PaginationSchemeObject::scheme_type` is a raw `Value` so an
  invalid type survives parsing to be reported by `validate`.
- `validate.rs` checks a scheme against the spec's own rules (§9); an
  invalid scheme is excluded from auto-detection rather than returned as
  an error — one broken scheme in a document shouldn't disable the rest.
- `autodetect.rs`'s `resolve_effective_scheme` picks the scheme that
  applies to an operation: an explicit `x-pagination` entry first, else
  auto-detection by matching declared query parameter/body field names
  (§6.2 default rules — a dimension with zero declared fields never
  vacuously matches). Only the query-parameter and body-field dimensions
  are implemented; `AutoDetectObject::match_headers`/`match_response_fields`
  and `RequestPaginationFieldsObject::header_fields` mirror the spec but
  nothing reads them yet, so a header-only scheme (e.g. a `Link`-header
  `nextLink` scheme, like GitHub's) needs an explicit `x-pagination` entry.
- `items.rs` locates which top-level response property holds the list of
  items — the extension only describes pagination metadata, not where
  items live.
- `request_builder.rs` builds query parameters for a page from a
  `PageCursor` and computes the next cursor. `next_step` wraps that with
  the two cases a cursor alone can't express — following a `nextLink`
  scheme's URL directly, and a `MAX_PAGES` cap so a misconfigured `Link`
  header can't spin a traversal forever.
- `response_parser.rs` parses that state back out of a response (dotted
  `bodyFields` paths, RFC 8288 `Link` parsing for `nextLink`-role headers)
  and derives `has_next_page`.

**Pagination is orthogonal to the collection/item resource model.** In
real APIs the paths that pair into a "resource" are often *not* the
paginated ones — real pagination usually lives on separate search/list
endpoints that have no sibling item path and are therefore invisible to
`discover_resources`.

### The sync engine (`src/sync/`)

A *different* surface from the four-stage pipeline above: not a port of
the TypeScript `syncables` package, but new scope tracked by
[issues #1–#9](https://github.com/localthought/syncables-rs/issues/1) — a
generic engine that reads an OpenAPI document plus a resource model
derived from its [CRUD Causality Extension](https://github.com/pondersource/openapi-extensions/tree/main/spec/crud-causality)
(`components.crudResources`), and syncs records into a host-provided
`Storage` implementation. [`localthought/reflector-rs`](https://github.com/localthought/reflector-rs)
is the first intended host; its `src/syncables.rs` is the contract this
module is written against, meant to be deleted once reflector-rs points
its `use`s here instead. **This crate must not depend on `atomic_lib`** —
records are plain JSON (`sync::storage::Record`) and the ontology is a
neutral description (`sync::ontology::Ontology`); rendering either into
Atomic Data is the host's job.

- **`resource_model.rs`** derives a `ResourceModel` from
  `components.crudResources`: one `ManagedCollection` per declared
  collection, with the resource's identity binding (a URL path variable
  need not be the payload's own `id` — GitHub's issues are addressed by
  `number`) and a `ContextProvider` for each path variable a nested
  collection needs from its parent's own records (e.g. `issueComments`'
  `issue_number`, from the parent `issues` collection's `number` field).
  Also reads the `x-crud` annotation off an operation (`crud_operation`)
  — action, resource, collection, write mode/`patchFormat`,
  `addedFields`, `memberOf`/`removesFrom` — which the write half of #9
  will need.
- **`constants.rs`** binds `ClientConfig::constants` into a resource
  model's path templates: `validate_constants` checks every constant
  names a declared parameter and every path variable is resolvable — by a
  constant, a parent record's `ContextProvider`, or a resource's own
  identity binding — before any request is made; `bind_url`
  percent-encodes values into a template.
- **`credentials.rs`** — `Credentials` (`Bearer`/`Anonymous`), whose
  `Debug` impl never renders the token, and `base_url`, reading the API's
  base URL from the document's own `servers` rather than separate
  configuration.
- **`ontology.rs`** derives an Atomic Data ontology from
  `components.crudResources`: one Class per resource, one Property per
  schema field — minted once and shared across every resource with a
  same-named field, not duplicated per resource. Terms carry only
  relative paths (no base URL): minting the actual public URL from
  `ClientConfig::ontology_base_url` is the host's job.
- **`storage.rs`** — the `Storage` trait
  (`put`/`get`/`list`/`delete`/`put_ontology`), keyed by
  `namespace`/`resource`/`id` so records sharing a resource name across
  different parents (every issue's comments are all `issueComment`) don't
  collide. `InMemoryStorage` is the reference implementation used by this
  crate's own tests.
- **`client.rs`** — `SyncClient::sync()`, the read half of #9: loads the
  document + overlays, derives the resource model, validates constants,
  derives and stores the ontology (before any record), then walks every
  managed collection — nested ones once per parent record, via a
  cartesian product across however many ancestor `ContextProvider`s a
  collection has — putting each record into `Storage`. Reuses
  `client::client::Fetch` for transport (the same injectable-transport
  design `ApiClient` already uses) rather than a parallel trait, so
  `SyncClient::new` takes an `Arc<dyn Fetch>` alongside `ClientConfig` —
  a deliberate divergence from the `reflector-rs` contract stub, which
  has no way to reach the network at all. One collection or one record
  failing is recorded in `SyncReport::errors`, not fatal to the rest of
  the sync. **Local-first writes (`create`/`update`/`remove`, the write
  queue, retry/backoff) are not implemented yet** — the remaining scope
  of #9.

## Conventions

- Rust 2021, `rustfmt` with `max_width = 100` (`rustfmt.toml`).
- Lints are configured in `Cargo.toml` under `[lints]`: `missing_docs`,
  `clippy::all` and `clippy::pedantic` are on. The handful of `allow`s
  there each carry a comment explaining why; prefer fixing a warning over
  adding another one.
- `module_inception` is allowed on purpose: the module tree deliberately
  mirrors the original's `src/client/client.ts` layout.
- Every public item needs a doc comment (`missing_docs` is a warning, and
  CI treats warnings as errors).
- Comments carried over from the TypeScript original are load-bearing
  documentation of *why* the code does what it does — keep them when you
  touch the code they describe.

## Tests

`tests/unit/` mirrors the original's `__tests__/unit/` layout module for
module. Cargo builds `tests/unit/main.rs` as a single test binary, so new
test files must be declared in its module tree — a file that isn't
declared there simply won't run.

`tests/fixtures/pets.rs` is the shared hand-written fixture (ported from
`__tests__/fixtures/pets.ts`). `tests/fixtures/real-world/` holds real
OpenAPI documents and pagination overlays vendored unmodified (see the
header comment in each file for provenance); the acceptance tests run the
pipeline against these and deliberately document real quirks rather than
working around them. When extending them, keep that spirit: assert what
actually happens against the unmodified real document, not an idealized
result.

## Generative AI disclosure (NLnet policy)

This project is NLnet-funded and follows [NLnet's Generative AI policy](https://nlnet.nl/foundation/policies/generativeAI/). If you (an AI coding assistant) make a commit here, keep it compliant:

- **Commit trailer.** Every commit produced with AI assistance carries a `Claude-Session: <url>` trailer (see the git commit instructions this session was given, or match the existing convention in `git log`).
- **Session log.** For any session that produces a commit, add or update a file under [`docs/ai-logs/sessions/`](docs/ai-logs/sessions) named `YYYY-MM-DD-<short-slug>.md`, following the format of existing entries there: session URL, model name/version, the substantive human prompts and substantive assistant outputs (not the harness's internal system prompt or tool-call plumbing — see [`docs/ai-logs/README.md`](docs/ai-logs/README.md) for what's in/out of scope).
- **Redact before committing.** Review the log for credentials, personal information (emails, etc.), and other session-identifying detail that isn't needed to understand what was asked and produced; mark redactions inline as `[redacted]` rather than deleting silently.
- **Keep the README in sync.** If how AI is used on this project changes in a way that makes the README's "Generative AI use" section inaccurate, update that section too.
- **Don't present AI output as unassisted human work,** and don't skip human review/testing before committing — both are conditions of the policy, not just house style.
