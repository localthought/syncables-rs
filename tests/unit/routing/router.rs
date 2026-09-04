use syncables::routing::router::find_route;

#[test]
fn matches_a_literal_path() {
    let templates = ["/pets", "/pets/{petId}"];
    let matched = find_route(&templates, "/pets").expect("matches");
    assert_eq!(matched.template, "/pets");
    assert!(matched.params.is_empty());
}

#[test]
fn binds_path_variables() {
    let templates = ["/pets", "/pets/{petId}"];
    let matched = find_route(&templates, "/pets/42").expect("matches");
    assert_eq!(matched.template, "/pets/{petId}");
    assert_eq!(matched.params.get("petId").map(String::as_str), Some("42"));
}

#[test]
fn percent_decodes_bound_variables() {
    let templates = ["/pets/{petId}"];
    let matched = find_route(&templates, "/pets/a%20b").expect("matches");
    assert_eq!(matched.params.get("petId").map(String::as_str), Some("a b"));
}

#[test]
fn does_not_match_across_segment_counts() {
    let templates = ["/pets/{petId}"];
    assert!(find_route(&templates, "/pets").is_none());
    assert!(find_route(&templates, "/pets/1/toys").is_none());
}

#[test]
fn returns_the_first_matching_template() {
    let templates = ["/pets/{petId}", "/pets/mine"];
    let matched = find_route(&templates, "/pets/mine").expect("matches");
    assert_eq!(matched.template, "/pets/{petId}");
}
