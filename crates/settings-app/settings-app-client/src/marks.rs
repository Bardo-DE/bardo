//! The marks of the providers the accounts service ships: each provider file names a mark and may give it a
//! face (a letter on a colour). The window draws them; this is only what the files say.

use account_provider::shipped_specs;

/// The letter and colour a provider's file gives its mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkFacts {
    /// The letter on the mark.
    pub letter: String,
    /// The colour behind it.
    pub colour: String,
}

/// What one shipped provider's file says about its mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMarkFacts {
    /// The provider's id.
    pub id: String,
    /// The named mark, as a word.
    pub mark: String,
    /// The face the file gives, if it does.
    pub face: Option<MarkFacts>,
}

/// The marks of the provider files the accounts service ships.
#[must_use]
pub fn shipped_marks() -> Vec<ProviderMarkFacts> {
    shipped_specs()
        .into_iter()
        .map(|spec| ProviderMarkFacts {
            id: spec.id.as_str().to_owned(),
            mark: str::to_owned(&spec.mark),
            face: spec.mark_face.as_ref().map(|face| MarkFacts {
                letter: face.letter.as_str().to_owned(),
                colour: face.colour.as_str().to_owned(),
            }),
        })
        .collect()
}
