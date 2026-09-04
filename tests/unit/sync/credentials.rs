use syncables::{base_url, Credentials, OpenApiDocument, ServerObject};

#[test]
fn bearer_credentials_render_an_authorization_header() {
    let credentials = Credentials::Bearer("ghp_secret".to_string());
    assert_eq!(
        credentials.authorization_header().as_deref(),
        Some("Bearer ghp_secret")
    );
}

#[test]
fn anonymous_credentials_have_no_authorization_header() {
    assert_eq!(Credentials::Anonymous.authorization_header(), None);
}

#[test]
fn debug_never_renders_the_bearer_token() {
    let credentials = Credentials::Bearer("do-not-leak-me".to_string());
    let rendered = format!("{credentials:?}");
    assert!(!rendered.contains("do-not-leak-me"));
    assert_eq!(rendered, "Bearer(<redacted>)");
}

#[test]
fn debug_of_anonymous_is_unremarkable() {
    assert_eq!(format!("{:?}", Credentials::Anonymous), "Anonymous");
}

#[test]
fn base_url_reads_the_first_declared_server() {
    let document = OpenApiDocument {
        servers: Some(vec![
            ServerObject {
                url: "https://api.github.com".to_string(),
                ..Default::default()
            },
            ServerObject {
                url: "https://api.staging.github.com".to_string(),
                ..Default::default()
            },
        ]),
        ..Default::default()
    };
    assert_eq!(base_url(&document), Some("https://api.github.com"));
}

#[test]
fn base_url_is_none_without_a_servers_list() {
    let document = OpenApiDocument::default();
    assert_eq!(base_url(&document), None);
}

#[test]
fn base_url_is_none_for_an_empty_servers_list() {
    let document = OpenApiDocument {
        servers: Some(vec![]),
        ..Default::default()
    };
    assert_eq!(base_url(&document), None);
}
