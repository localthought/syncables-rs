//! Runs the ported pipeline against real, unmodified OpenAPI documents.
//!
//! These deliberately document real quirks rather than working around
//! them. When extending them, keep that spirit: assert what actually
//! happens against the unmodified real document, not an idealized result.

use std::path::{Path, PathBuf};

use syncables::pagination::items::locate_items_field;
use syncables::{
    apply_overlay, discover_resources, load_open_api_document, load_overlay,
    resolve_effective_scheme, OpenApiDocument,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/real-world")
        .join(name)
}

/// Loads a real document and applies its vendored pagination overlay.
async fn overlaid(document: &str, overlay: &str) -> OpenApiDocument {
    let raw = load_open_api_document(fixture(document).as_path())
        .await
        .expect("document loads");
    let overlay = load_overlay(fixture(overlay).as_path())
        .await
        .expect("overlay loads");
    let raw = serde_json::to_value(raw).expect("document round-trips");
    let merged = apply_overlay(&raw, &overlay).expect("overlay applies");
    serde_json::from_value(merged).expect("overlaid document still fits the type surface")
}

#[tokio::test]
async fn loads_giphy_and_resolves_its_pagination_scheme() {
    let document = overlaid("giphy.yaml", "giphy-pagination-overlay.yaml").await;

    // Giphy's overlay originally declared `type: offset`, which is not a
    // valid scheme type; the vendored copy reflects the upstream fix.
    let schemes = document
        .components
        .as_ref()
        .and_then(|c| c.pagination_schemes.as_ref())
        .expect("the overlay added paginationSchemes");
    assert!(!schemes.is_empty());

    // Real pagination lives on a search/listing endpoint that has no
    // sibling item path, so it is invisible to `discover_resources`.
    let trending = document
        .paths
        .get("/gifs/trending")
        .and_then(|item| item.get.as_ref())
        .expect("giphy documents /gifs/trending");
    let resolved =
        resolve_effective_scheme(&document, trending).expect("a scheme applies to /gifs/trending");
    assert!(schemes.contains_key(&resolved.scheme_name));

    // Its response is enveloped: the items live under a named property.
    let items_field = trending
        .responses
        .get("200")
        .and_then(|response| response.content.as_ref())
        .and_then(|content| content.get("application/json"))
        .and_then(|media| media.schema.as_ref())
        .and_then(|schema| locate_items_field(Some(schema), Some(&resolved.scheme)));
    assert_eq!(items_field.as_deref(), Some("data"));
}

#[tokio::test]
async fn loads_spotify_and_resolves_a_scheme_on_a_listing_endpoint() {
    let document = overlaid("spotify.yaml", "spotify-pagination-overlay.yaml").await;

    let albums = document
        .paths
        .get("/artists/{id}/albums")
        .and_then(|item| item.get.as_ref())
        .expect("spotify documents /artists/{id}/albums");
    let resolved = resolve_effective_scheme(&document, albums)
        .expect("a scheme applies to /artists/{id}/albums");
    assert!(!resolved.scheme_name.is_empty());
}

#[tokio::test]
async fn discovers_resources_in_a_real_document() {
    let document = load_open_api_document(fixture("giphy.yaml").as_path())
        .await
        .expect("document loads");
    // Whatever the count, discovery must not panic or hang on a real
    // document, and every route it returns must be well-formed.
    for route in discover_resources(&document.paths) {
        assert!(route.collection_path.starts_with('/'));
        assert!(route.item_path.starts_with(&route.collection_path));
        assert!(!route.item_param.is_empty());
    }
}
