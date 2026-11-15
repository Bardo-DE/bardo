//! `application/x-www-form-urlencoded`, through `form_urlencoded`: the pairs of a query or a form
//! body, and a body made of pairs. It names nothing else of this crate.

/// The pairs of a query string or form body, decoded: `%XX` and `+` as a space, a malformed
/// escape kept as written so a truncated code fails at the issuer and not silently here. A part
/// without `=` is a key with an empty value.
pub(crate) fn pairs(query: &str) -> Vec<(String, String)> {
    form_urlencoded::parse(query.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

/// Pairs as a form body: a space is `+`, every other byte but `A-Z a-z 0-9 * - . _` is `%XX` (so `~` is `%7E`).
pub(crate) fn body(pairs: &[(&str, &str)]) -> String {
    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
}
