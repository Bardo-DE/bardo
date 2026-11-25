//! Reading rendered markup without depending on the order the renderer writes attributes in.

/// The start tag (`<tag ...>`) of the first element whose `class` attribute is exactly `class`.
pub fn start_tag<'a>(html: &'a str, class: &str) -> Option<&'a str> {
    let at = html.find(&format!("class=\"{class}\""))?;
    let from = html[..at].rfind('<')?;
    let to = at + html[at..].find('>')?;
    Some(&html[from..=to])
}

#[test]
fn a_start_tag_is_found_by_its_class_whatever_the_attribute_order() {
    let html = r#"<div><span aria-hidden="true" class="pic" data-size="64"></span></div>"#;
    assert_eq!(
        start_tag(html, "pic"),
        Some(r#"<span aria-hidden="true" class="pic" data-size="64">"#)
    );
    assert_eq!(start_tag(html, "pi"), None);
}
