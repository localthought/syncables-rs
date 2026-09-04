//! Building the query for one page request, and computing the next cursor.

use super::types::{PaginationQuery, PaginationResponseState, PaginationSchemeObject, SchemeType};

/// Where the client is in a paginated traversal, independent of scheme type.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PageCursor {
    /// Item offset, for offset-based schemes.
    pub offset: Option<u64>,
    /// Page number, for page-based schemes.
    pub page: Option<u64>,
    /// Opaque token, for token/cursor-based schemes.
    pub page_token: Option<String>,
}

fn fields_with_role(scheme: &PaginationSchemeObject, role: &str) -> Vec<String> {
    scheme
        .request
        .as_ref()
        .and_then(|request| request.query_parameters.as_ref())
        .into_iter()
        .flatten()
        .filter(|(_, field)| field.role.as_deref() == Some(role))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Builds the query parameters for one page request from the current cursor.
pub fn build_query(
    scheme: &PaginationSchemeObject,
    cursor: &PageCursor,
    page_size: Option<u64>,
) -> PaginationQuery {
    let mut query = PaginationQuery::new();

    if let Some(page_size) = page_size {
        for name in fields_with_role(scheme, "pageSize") {
            query.insert(name, page_size.to_string());
        }
    }
    for name in fields_with_role(scheme, "offset") {
        query.insert(name, cursor.offset.unwrap_or(0).to_string());
    }
    for name in fields_with_role(scheme, "page") {
        query.insert(name, cursor.page.unwrap_or(1).to_string());
    }
    if let Some(page_token) = &cursor.page_token {
        for name in fields_with_role(scheme, "pageToken")
            .into_iter()
            .chain(fields_with_role(scheme, "cursor"))
        {
            query.insert(name, page_token.clone());
        }
    }

    query
}

/// Computes the cursor for the next page from the previous cursor and the
/// parsed response state.
///
/// Returns `None` when there is no next page, or when the scheme type is
/// `nextLink` — for that type the caller should follow
/// [`PaginationResponseState::next_link`] directly rather than rebuilding
/// query parameters.
pub fn next_cursor(
    scheme: &PaginationSchemeObject,
    cursor: &PageCursor,
    state: &PaginationResponseState,
    items_returned: u64,
) -> Option<PageCursor> {
    if !state.has_next_page {
        return None;
    }

    match scheme.typed()? {
        SchemeType::PageToken => state.next_page_token.clone().map(|token| PageCursor {
            page_token: Some(token),
            ..PageCursor::default()
        }),
        SchemeType::NextLink => None,
        SchemeType::PageNumber => {
            if !fields_with_role(scheme, "offset").is_empty() {
                return Some(PageCursor {
                    offset: Some(cursor.offset.unwrap_or(0) + items_returned),
                    ..PageCursor::default()
                });
            }
            if !fields_with_role(scheme, "page").is_empty() {
                return Some(PageCursor {
                    page: Some(cursor.page.unwrap_or(1) + 1),
                    ..PageCursor::default()
                });
            }
            None
        }
    }
}
