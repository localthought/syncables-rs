# Session log — 2026-09-04

- **Session:** https://claude.ai/code/session_01BWbBQnekeiUnnqzfdyVQTF
- **Model:** Sonnet 5 (`claude-sonnet-5`)
- **Repos touched:** `localthought/syncables-rs` (this repo). `localthought/reflector`
  and `localthought/reflector-rs` were cloned read-only for reference: the
  former holds the TypeScript overlay-target tokenizer this change ports,
  the latter holds the `ClientConfig`/overlay fixtures the issue's "test
  material" section points at.
- **Redactions applied:** none — no credentials or personal information
  appeared in this session's substantive turns.

This log records the substantive human prompts and the assistant's
substantive outputs, per the scoping explained in
[`docs/ai-logs/README.md`](../README.md). It omits the coding assistant's
internal system prompt, tool-call plumbing, and other harness scaffolding.

---

## Turn 1

**User prompt:**

> work on open issues

**Assistant output (summarized):**

Listed the repository's 9 open issues (#1–#9), all part of a single
umbrella epic — porting reflector-rs's sync engine (`SyncClient`,
`ClientConfig`, resource-model derivation, pagination, credentials,
storage, ontology) — tracked as a dependency chain from #1. Checked open
PRs and found #11 already addressing #7 (`Storage` trait), so avoided
duplicating that work.

Picked issue #2, "Load an OpenAPI document and apply an ordered list of
OpenAPI Overlays," as the most foundational unblocked piece: everything
else in the chain depends on #2 (directly or transitively) and nothing
else was in flight against it.

Cloned `localthought/reflector` and `localthought/reflector-rs` read-only
to ground the implementation in the actual reference behaviour named in
the issue: `src/sync/overlay.ts`'s `parseTarget`/`applyOverlay` (the
bracket-target tokenizer), `src/sync/document.ts`'s `buildDocumentFrom`
(load-then-overlay ordering, relative to `$ref` resolution), and the real
GitHub crud-causality/pagination/auth overlay fixtures under
`reflector-rs/spec/overlays/github/`, which target operations like
`$.paths['/repos/{owner}/{repo}/issues'].post` — exactly the bracketed
form the crate's overlay parser rejected outright before this change.

Implemented, in `syncables-rs`:

- `src/openapi/overlay.rs`: rewrote `parse_target` from a `$.` + `.split('.')`
  parser (which rejected any target containing `[`) into a tokenizer
  supporting `$`, dot-path segments, and single- or double-quoted bracket
  segments (`$.paths['/pets/{petId}'].get`), ported from the TypeScript
  `parseTarget` in `reflector/src/sync/overlay.ts`. Wildcards, unquoted
  bracket keys, and unterminated brackets remain rejected with
  `Error::UnsupportedOverlayTarget`.
- Added `load_open_api_document_with_overlays`, composing
  `load_open_api_document` (which resolves `$ref`s) with `load_overlay` +
  `apply_overlay` over an ordered list of overlay paths — mirroring
  `buildDocumentFrom`'s load-then-overlay sequencing.
- `src/openapi/load.rs`: factored the path-read-and-parse branch shared by
  `load_open_api_document` and `load_overlay` into `load_yaml_file`.
- `src/error.rs`: added `Error::FileLoad { path, source }`, so a missing or
  unparsable document/overlay file names the offending path in its error
  message — a bare `std::io::Error` from `read_to_string` doesn't otherwise
  mention which file failed, which the issue calls out explicitly ("a
  missing document ... should surface ... with the offending file ...
  named").
- Updated `CLAUDE.md`'s architecture section (the "plain dot-path targets"
  line, now stale) and `src/lib.rs`'s re-exports.
- Added unit tests for bracket-target parsing (quoted, double-quoted,
  unterminated, unquoted) in `tests/unit/openapi/overlay.rs`, and
  acceptance tests in `tests/unit/acceptance/real_world.rs` exercising
  `load_open_api_document_with_overlays` against the vendored Giphy
  fixtures, plus tests asserting a missing document/overlay path is named
  in the resulting error.

Ran `cargo fmt --all --check`, `cargo clippy --all-targets --all-features
-- -D warnings`, `cargo test`, and `cargo build --release` — the same four
checks CI runs — all green (79 tests passing, up from 71).

Opened [PR #12](https://github.com/localthought/syncables-rs/pull/12) and
subscribed to its activity. CI passed and it merged cleanly with no review
comments; unsubscribed and cleaned up the scheduled check-in once merged.

## Turn 2

**User prompt:**

> merge at will and continue working

**Assistant output (summarized):**

Merged PR #12 (squash, matching the one precedent in `git log` — PR #10
was squash-merged too) and unsubscribed from its activity.

Synced `main` and checked PR #11 (`claude/issue-7-storage-trait`, open,
addressing issue #7) to see what it had already established before picking
the next issue, since it's the first of the #1–#9 series to land any code:
a new top-level `src/sync/` module, distinct from the crate's existing
`syncables`-TypeScript-port surface, with its own module doc explaining
that split. Followed the same convention rather than inventing a
different location for new engine code.

Picked issue #5, "Credentials: static bearer token now, document-derived
OAuth later" — unblocked (only depends on #2, now merged) and, unlike #3/#4/#9,
self-contained enough not to presuppose design decisions (a transport/HTTP
layer, the resource model) that belong to whichever issue builds them.

Read `localthought/reflector`'s `src/oauth/static-token.ts`
(`StaticTokenManager`) and `src/oauth/authed-fetch.ts` to see what the
credential actually has to do in the original: attach `Authorization:
Bearer <token>` and retarget a request at the real API base derived from
the document's `servers` — never a separately configured URL.

Implemented, in `syncables-rs`:

- `src/openapi/types.rs`: added `ServerObject { url, extensions }` and an
  optional `servers` field on `OpenApiDocument` (previously undeclared,
  falling into the `extensions` catch-all).
- `src/sync/mod.rs` (new): the module PR #11 introduced doesn't exist on
  `main` yet (that PR is unmerged), so this session created it, mirroring
  #11's own doc comment and adjusting it to name `credentials` as what
  exists so far in this branch.
- `src/sync/credentials.rs` (new): `Credentials` (`Bearer(String)` /
  `Anonymous`), ported field-for-field from the `Credentials` enum in
  `reflector-rs/src/syncables.rs`'s contract stub, including the redacted
  `Debug` impl (`Bearer(<redacted>)`, never the token). Added
  `Credentials::authorization_header()` and a `base_url(document)` function
  reading `document.servers.first()`.
- `src/lib.rs`: registered `pub mod sync;`, re-exported `ServerObject`,
  and re-exported `Credentials`/`base_url` from `sync::credentials`.
- 7 new tests in `tests/unit/sync/credentials.rs`: header rendering for
  both variants, `Debug` redaction (asserting the raw token string does
  not appear in the rendered output), and `base_url` against a populated,
  absent, and empty `servers` list.

Did not implement `securitySchemes` parsing/validation: the issue's actual
milestone behaviour is "always send bearer if configured," and validating
a scheme this crate doesn't yet use anywhere felt like scope the issue
didn't ask for. Left `SyncError`/`ClientConfig`/`SyncClient` alone too —
that's #1's/#9's assembly, not this issue's.

Ran the same four CI checks locally — all green (86 tests passing, up from
79).

Opened [PR #13](https://github.com/localthought/syncables-rs/pull/13),
subscribed, and — once CI passed — merged it (squash) per the standing
"merge at will" instruction from Turn 2, then unsubscribed.

## Turn 3

**Prompt:** none — this turn continues autonomously from Turn 2's "merge at
will and continue working," per the session's Auto Mode instructions.

**Assistant output (summarized):**

Synced `main`, restarted the branch from it, and picked issue #3, "Derive
the resource model from the `crudResources` overlay extension" — unblocked
(depends only on #2) and central: #6 and #9 both depend on it.

Read `localthought/reflector`'s `src/sync/resources.ts`
(`discoverResourceModel`), the reference the issue names, and the actual
GitHub crud-causality overlay in `localthought/reflector-rs`'s
`spec/overlays/github/crud-causality-overlay.yaml` to ground the
implementation in real shapes rather than the issue text alone. Noted that
`resources.ts` is written for Calendar (adds a client-generated-id policy
GitHub's server-assigned `addedFields` never need) and that the issue
additionally describes per-operation `x-crud` (action/mode/patchFormat/
addedFields/memberOf/removesFrom) that `resources.ts` mostly doesn't read
either — so treated the issue's own text as the spec and `resources.ts` as
the reference for the parts it actually covers (identity bindings,
context-parameter resolution across nested collections).

Implemented `src/sync/resource_model.rs` (new):

- Raw typed structs for `components.crudResources`'s shape
  (`CrudResourceObject`/`ResourceIdentityObject`/`IdentityBindingObject`/
  `ResourceCollectionObject`), read from the document's `extensions` catch-all
  rather than added to `ComponentsObject` — this is CRUD-causality-extension
  territory, not core OpenAPI, so it stays inside `src/sync/` rather than
  widening the base type surface in `openapi/types.rs`.
- `discover_resource_model(document) -> Result<ResourceModel>`: one
  `ManagedCollection` per declared collection, across every resource,
  porting `resources.ts`'s two load-bearing details — a resource's URL
  identity need not be its payload's own `id` (GitHub issues: `number`),
  and a nested collection's unresolved path variable (`issue_number` on
  `issueComments`) is resolved via a `ContextProvider` pointing at the
  parent collection/field, not left dangling.
- `CrudOperation`/`CrudAction`/`AddedField`/`CollectionMembership` +
  `crud_operation(operation)`, reading the `x-crud` block per operation —
  action, resource, collection, update mode/patchFormat, server-added
  fields, and collection membership (`memberOf`/`removesFrom`, the latter
  either a named list or `"*"`, hand-written `Deserialize` since that's two
  JSON shapes for one field).
- `Error::NoCrudResources` for a document with no `crudResources` at all.
- `src/lib.rs`/`src/sync/mod.rs`: registered and re-exported the module.
- 12 new tests in `tests/unit/sync/resource_model.rs`, built from a
  synthetic document shaped exactly like the real GitHub overlay's own
  example: collection derivation, the `number`-not-`id` identity case, the
  nested-collection provider resolution, constant-only params having no
  provider, every `x-crud` action shape, and the malformed-`removesFrom`
  error path.

Also vendored `localthought/reflector-rs`'s `spec/github-issues.openapi.yaml`
+ its three overlays to write an end-to-end acceptance test (load + apply
overlays + derive the resource model, against real GitHub-shaped material) —
but the base document failed to parse: its `body`/`state_reason` schemas use
JSON Schema's `type: [string, "null"]` array form for nullability, which
`SchemaObject::schema_type` (typed `Option<String>`) can't represent. Fixing
that touches `fake_data`/`pagination`, modules CLAUDE.md marks "ported and
tested," for a concern unrelated to #3 — rather than widen this PR into that,
removed the vendored fixture and acceptance test, and filed
[#14](https://github.com/localthought/syncables-rs/issues/14) documenting
the gap for whoever picks it up (with a suggested `SchemaType` enum shape).
The synthetic-JSON tests already cover #3's logic thoroughly on their own.

Ran the same four CI checks locally — all green (97 tests passing, up from
86).

Opened [PR #15](https://github.com/localthought/syncables-rs/pull/15),
subscribed, and — once CI passed — merged it (squash), then unsubscribed.

## Turn 4

**Prompt:** none — continues autonomously, same standing instruction.

**Assistant output (summarized):**

Synced `main`, restarted the branch, and picked issue #6, "Constants: bind
configured values into path and query parameters" — now unblocked (depends
on #3, just merged) and a natural continuation of the resource-model work
still in context.

Implemented `src/sync/constants.rs` (new):

- `validate_constants(document, model, constants)`: checks every constant
  key names a parameter the document actually declares (path or query, on
  any operation) — `Error::UnknownConstant` otherwise, so a typo'd
  `repository=` can't silently widen a sync to the whole account rather
  than one repo. Then checks every path variable a managed collection
  needs is resolvable — by a constant, a parent record's `ContextProvider`
  (from #3), or the resource's own identity binding — `Error::UnboundContextParam`
  otherwise. The identity-binding case needed a small extension to #3's
  `ManagedCollection`: a new `identity_params` field (the item URL's own
  self-bound path variables, e.g. `issue_number` for `issue`) so
  `issueComments`' `{comment_id}` isn't wrongly flagged as needing outside
  resolution.
- `bind_url(template, values)`: substitutes `{param}` segments,
  percent-encoding each value (a hand-rolled path-segment `AsciiSet` via
  the existing `percent-encoding` dependency, mirroring the `url` crate's
  `PATH_SEGMENT_ENCODE_SET` rather than adding that crate for one constant).
- Two new `Error` variants (`UnknownConstant`, `UnboundContextParam`) and a
  `pub(super)` on `resource_model::path_variables` so `constants.rs` could
  reuse it instead of re-implementing template parsing.
- 9 new tests in `tests/unit/sync/constants.rs`: valid owner/repo binding,
  the undeclared-constant error, the unbound-variable error, the
  nested-collection-provider and own-identity-binding cases both needing no
  constant, percent-encoding on bind, an unbound bind_url call, and the
  no-variables-to-bind pass-through.

`x-list-query`'s "applied to every list request" behaviour was already
delivered by #3's `ManagedCollection::list_query`, so nothing new was
needed there beyond what #3 already tests.

Ran the same four CI checks locally — all green (105 tests passing, up
from 97).

Opened [PR #16](https://github.com/localthought/syncables-rs/pull/16),
subscribed, and — once CI passed — merged it (squash), then unsubscribed.

## Turn 5

**Prompt:** none — continues autonomously, same standing instruction.

**Assistant output (summarized):**

Synced `main`, restarted the branch, and picked issue #8, "Derive an
Atomic Data ontology from the document and mint its terms under a
host-supplied public URL." Unblocked (depends on #2, #3, both merged).

Read `reflector-rs/src/ontology.rs` (`SubjectMapper`) and `src/store.rs`
(`AtomicStorage`, its hand-built test `Ontology` fixture) to confirm the
boundary already settled by the `ClientConfig`/`Ontology`/`OntologyTerm`
contract in `reflector-rs/src/syncables.rs`: this crate emits relative
*paths* only (`github-issues/property/title`, no leading slash, no base
URL) — minting the actual `internal:`/public URL pair from
`ClientConfig::ontology_base_url` is entirely the host's job
(`SubjectMapper`), confirming `derive_ontology` doesn't need a base-URL
parameter at all, just the document.

The issue leaves two things genuinely unspecified — no TS reference exists
for this one (it's new to the Rust port) and the given contract stub
doesn't say: where `Ontology.path`/`.shortname`/`.description` themselves
come from (as opposed to individual terms', which the issue does specify).
Decided: derived from `document.info.title`, slugified — flagged this as a
judgment call in the PR description rather than silently guessing.

Implemented `src/sync/ontology.rs` (new):

- `derive_ontology(document) -> Result<Ontology>`: one Class per
  `crudResources` entry, one Property per field of that resource's schema
  — minted once and *shared* across every resource with a same-named field
  (e.g. `body` on both `issue` and `issueComment` is one property term, not
  two), matching how RDF-style vocabularies reuse properties across
  classes.
- `slugify`: lowercase, `-`-separated, collapsing runs of non-alphanumeric
  characters (`state_reason` -> `state-reason`); a `claim_shortname` guard
  that reuses a slug for the *same* original name but errors
  (`Error::ShortnameCollision`) if a *different* name would collide onto
  it — validated before ever consulting the property-reuse map, so two
  different fields that happen to slugify identically can't be silently
  merged.
- A small `datatype_url` mapping (string/integer/number/boolean, plus
  `string`+`format:date-time`/`date`) to `https://atomicdata.dev/datatypes/*`
  URLs, omitting anything it can't place rather than guessing, per the
  issue's own instruction.
- `resource_schema`: resolves `crudResources.<resource>.schema`'s `$ref` by
  name against `components.schemas`. This `$ref` is a real quirk worth
  flagging — the overlay adds it via `update` *after* `load_open_api_document`
  already ran `resolve_refs`, so it's never automatically inlined; had to
  resolve it by hand here rather than relying on the crate's existing ref
  resolution.
- `requires`/`recommends` from the schema's `required` list vs. the rest of
  its properties, as property *paths* (cross-reference resolution by origin
  is explicitly the host's job per the issue).
- `pub(super)` on `resource_model::crud_resources` so `ontology.rs` could
  reuse the same `crudResources` reader instead of duplicating it.
- 8 new tests in `tests/unit/sync/ontology.rs`: class derivation, snake_case
  shortname normalization, the datatype mapping (including the
  `date-time`->`timestamp` case), required-vs-recommended placement, the
  shared-property-not-duplicated case (asserting exactly one `body` term
  across two resources, and that its home resource's field-required-ness
  still differs per class), a genuine collision
  (`state_reason`/`state-reason` on one resource), and an unmapped type
  (`array`) correctly omitting `datatype`.

Ran the same four CI checks locally — all green (113 tests passing, up
from 105).

Opened [PR #17](https://github.com/localthought/syncables-rs/pull/17),
subscribed, and — once CI passed — merged it (squash), then unsubscribed.

## Turn 6

**Prompt:** none — continues autonomously, same standing instruction.

**Assistant output (summarized):**

Checked remaining open issues: #4 (pagination) was the only unblocked,
unclaimed one left — #7's PR #11 is still open and not this session's to
touch, #1/#9 are the umbrella/final-assembly issues that presuppose the
others. Synced `main`, restarted the branch, and picked #4.

Read the existing `src/pagination/` module (already "ported and tested"
per `CLAUDE.md`, predating the #1-9 series) closely against the issue's
own text before writing anything, since most of what #4 describes turned
out to already exist: `autodetect::resolve_effective_scheme` already
checks `x-pagination` before falling back to auto-detection;
`response_parser::parse_link_header` already handles multiple `rel`
link-values in one header and quoted `rel="next"` params (`split_links`,
`rel_is_next`); `request_builder::next_cursor` already documents that a
`nextLink` scheme's next page should be followed as a URL rather than
rebuilt from the template; token-in-query (`pageToken`/`nextPageToken`/
`cursor` roles) was already fully wired in both `request_builder.rs` and
`response_parser.rs`.

The one genuinely missing piece was "a page limit / total cap, so a
misconfigured `Link` header cannot spin forever" — there was no walking
loop yet, `next_cursor` alone can't express "stop, we've walked enough"
or "here's the literal URL to follow next." Rather than build a full
transport/HTTP abstraction to drive that loop (which would presuppose
design decisions on #9's turf, the way PR #11 flagged doing for
`Storage`), scoped this down to what #4 itself is missing: a
pure decision function.

Added to `src/pagination/request_builder.rs` (not `src/sync/` — this has
no coupling to `crudResources`/`ClientConfig`, it belongs with the
existing generic pagination primitives it wraps):

- `MAX_PAGES: usize = 50`, mirroring `client::client::MAX_PAGES` (the
  equivalent constant already sitting unused next to the still-`todo!()`
  `ApiClient::paginate`).
- `PageStep` (`FollowLink(String)` / `NextPage(PageCursor)` / `Done`) and
  `next_step(scheme, cursor, state, items_returned, pages_fetched)`,
  wrapping `next_cursor` with the two cases it deliberately doesn't
  handle: a `nextLink` scheme (whose next page is a URL, not a cursor —
  `next_cursor` already returns `None` for that type on purpose) and the
  `MAX_PAGES` cap.
- Updated `CLAUDE.md`'s pagination section to describe it.
- 6 new tests in `tests/unit/pagination/request_builder.rs`: no-next-page
  stops, an offset scheme advances normally, a `nextLink` scheme returns
  the literal URL rather than a cursor, a `nextLink` scheme with
  `has_next_page` set but no link present still stops (rather than
  panicking or looping), the cap stops a traversal that would otherwise
  continue forever, and one page short of the cap does not stop early.

Ran the same four CI checks locally — all green (119 tests passing, up
from 113).

Opened [PR #18](https://github.com/localthought/syncables-rs/pull/18),
and — once CI passed — merged it (squash). Checked PR #11 (issue #7,
`Storage` trait, from a different session): still open, `mergeable_state:
"dirty"` (a real conflict against `main`, now several commits ahead of
that PR's stale base). Not this session's PR to touch uninvited, so
reported status and stopped rather than fixing it unasked.

## Turn 7

**User prompt:**

> feel free to fix the merge conflict in pr #11 and merge that, then
> continue working

**Assistant output (summarized):**

Checked out `claude/issue-7-storage-trait` (PR #11's branch, authored by a
different session) and merged `origin/main` into it with a regular merge
commit — never rebase or force-push someone else's branch. Three
conflicts, all in files this session had itself extended since #11
branched: `src/sync/mod.rs`, `src/lib.rs`, `tests/unit/main.rs`. Two were
purely additive (both sides adding sibling `pub mod`/re-export/test-module
lines); one was a real collision worth resolving with judgment rather than
picking a side: `sync::storage::Ontology` was an explicit placeholder unit
struct ("this crate's issue #8 defines this type's real shape... until
that lands, this is an empty marker"). #8 had since landed (this session's
own PR #17) with the real `Ontology` (`path`/`shortname`/`description`/
`terms`), so `sync::storage` now imports that instead of redefining its
own — updated `Storage::put_ontology`/`InMemoryStorage` and the storage
tests accordingly (the real `Ontology` is `Clone` but not `Copy`, unlike
the placeholder, so `push(*ontology)` became `push(ontology.clone())`,
and the tests' bare `Ontology` unit values became a constructed
`Ontology { path, shortname, description, terms: vec![] }`).

Ran the same four checks (130 tests, up from 119 — PR #11's own 12 tests
now included) and pushed the merge commit — a normal, non-force push,
since it fast-forwards from the branch owner's last commit. CI passed;
merged PR #11 (squash), closing #7.

With #7 merged, only #1 (crate skeleton) and #9 (`client.sync()`) remained
open in the #1–#9 series (plus #14, the unrelated `schema_type` gap filed
earlier). Rather than start either autonomously, laid out the concrete
design decisions #9 actually turns on — reusing `client::client::Fetch`
for transport vs. forking a new trait, scoping to the read path only vs.
also building local-first writes, whether `ClientConfig`/`SyncClient`/
`SyncReport`/`SyncError` should copy the `reflector-rs` contract stub
verbatim, and the partial-failure model — and asked before writing code.

**User prompt:**

> 1) that's fine
> 2) yes, let's stick to read-only and leave local-first writes for later
> 3) yes, copy them from the reflector-rs stub, and then we'll delete that
> stub later when reflector-rs starts consuming syncables-rs.
> 4) OK

**Assistant output (summarized):**

Synced `main`, restarted the branch, and implemented the read-only half of
#9 in a new `src/sync/client.rs`:

- `ClientConfig`/`SyncReport`/`SyncError` copied field-for-field from
  `reflector-rs/src/syncables.rs`'s contract stub, with one flagged,
  agreed divergence: that stub's `SyncClient::new` takes only a
  `ClientConfig`, with no way to reach the network at all. Since this
  crate has no HTTP client dependency and (per decision 1) reuses
  `client::client::Fetch` — the same injectable-transport design the
  existing `ApiClient` already uses — `SyncClient::new` additionally takes
  an `Arc<dyn Fetch>`.
- `SyncClient::sync()`: loads the document + overlays, derives the
  resource model, validates constants, resolves the base URL — failing
  before any write if the document declares no `servers`, so a doomed
  sync never even writes an ontology it can't back with data — then
  derives and stores the ontology, then walks every managed collection.
- The walk itself is the piece with no existing precedent to port: a
  retry-until-fixed-point loop over `ResourceModel::collections`, where a
  collection with no parent-provided context parameter walks once (using
  only `ClientConfig::constants`), and one with a `ContextProvider`
  dependency walks once per record of its (by-then-already-read) parent —
  `binding_combinations` builds the cartesian product across however many
  ancestor providers a collection has, so nesting more than one level deep
  falls out of the same code rather than needing special-casing. Each
  collection's own page-by-page walk reuses #4's `next_step`/`PageCursor`
  entirely unmodified.
- Namespace and record id: `namespace` is the collection's own
  `context_params` values joined by `/` (so a root collection's records
  get e.g. `"acme/widgets"`, and a nested one's get `"acme/widgets/1"` —
  the issue doesn't pin this down exactly beyond "derived from the
  resource model's context parameters," so this is another flagged
  judgment call, chosen to match the shape of the worked namespace example
  already in #7's own doc comment); record id comes from
  `ManagedCollection::id_field`.
- Partial failure: one collection's request failing (network error,
  non-2xx status, unparseable body) or one record's `Storage::put` failing
  is caught and pushed into `SyncReport::errors` as `"<collection>:
  <message>"`, and the walk continues — never propagated as a fatal
  `SyncError`.
- 9 new tests in `tests/unit/sync/client.rs`, against a hand-written
  `MockFetch` test double (serves a fixed response per exact URL, records
  every URL requested) rather than a live server: a flat collection sync,
  a `nextLink`-paginated collection walked across two pages, a nested
  collection walked once per parent record with per-parent namespacing
  verified, a 404 on a nested collection recorded as a partial failure
  while the rest of the sync still succeeds, a `Storage::put` failure
  likewise partial-not-fatal, `SyncClient::new` rejecting an empty
  `ontology_base_url`, `sync()` erroring when the document declares no
  servers (and writing nothing first), `sync()` erroring when a constant
  is unresolvable, and one asserting the exact set of URLs requested.

Ran the same four CI checks locally — all green (139 tests passing, up
from 130).
