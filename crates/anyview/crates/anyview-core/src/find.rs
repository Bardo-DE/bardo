//! What a find is told besides its text: whether letter case and whole words matter. Text and PDF
//! search read the same options, and the viewer's find keeps one set for the window.

/// How a find matches. Start from [`FindOptions::default`] (ignore case, find parts of words) and
/// change one with the `with_*` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FindOptions {
    /// Letter case matters: `Fox` finds only `Fox`, not `fox`.
    pub match_case: bool,
    /// A hit is a whole word: `fox` does not find the start of `foxes`.
    pub whole_words: bool,
}

impl FindOptions {
    /// Ignore case and find parts of words: what [`Default`] answers, as a constant for a `const`
    /// that changes only some options.
    pub const DEFAULT: FindOptions = FindOptions {
        match_case: false,
        whole_words: false,
    };

    /// These options with `match_case`.
    #[must_use]
    pub const fn with_match_case(self, match_case: bool) -> Self {
        FindOptions { match_case, ..self }
    }

    /// These options with `whole_words`.
    #[must_use]
    pub const fn with_whole_words(self, whole_words: bool) -> Self {
        FindOptions {
            whole_words,
            ..self
        }
    }
}
