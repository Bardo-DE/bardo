//! A recording as a player describes it: its tracks, its chapters, its tags and whether a picture
//! shows. Plain data, so the player's back end, the viewer's stage and the desktop's now-playing
//! entry all say the same things the same way.

use crate::units::MediaTime;
use ui_core::word::Word;

/// Which family of stream a track belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum StreamKind {
    /// Moving pictures, or the cover image of an audio file.
    Video,
    /// Sound.
    Audio,
    /// Subtitles.
    Subtitles,
}

/// One track of a recording.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaTrack {
    /// The track's number within its kind; the first of each kind is 1.
    pub id: u32,
    /// Which family of stream it is.
    pub kind: StreamKind,
    /// The title tag, when the file has one.
    pub title: Option<String>,
    /// The language tag, usually an ISO 639 code.
    pub language: Option<String>,
    /// The codec's name, such as `h264` or `flac`.
    pub codec: Option<String>,
    /// Whether it is the track playing now: decoded and presented.
    pub playing: bool,
}

impl MediaTrack {
    /// How a list shows the track: its title, else its language, else "Track N".
    #[must_use]
    pub fn name(&self) -> String {
        match (&self.title, &self.language) {
            (Some(title), Some(language)) => format!("{title} ({language})"),
            (Some(name), None) | (None, Some(name)) => name.clone(),
            (None, None) => format!("Track {}", self.id),
        }
    }
}

/// A chapter mark.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaChapter {
    /// The title; empty when the file does not name it.
    pub title: String,
    /// Where the chapter starts.
    pub start: MediaTime,
}

/// Whether the recording shows a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum VideoPresence {
    /// No picture: audio only, or nothing loaded.
    #[default]
    Absent,
    /// A still cover image attached to an audio file.
    CoverArt,
    /// Moving pictures.
    Present,
}

/// What a file says of itself in its tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MediaTags {
    /// The title tag.
    pub title: Option<String>,
    /// The artist tag.
    pub artist: Option<String>,
    /// The album tag.
    pub album: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: Option<&str>, language: Option<&str>) -> MediaTrack {
        MediaTrack {
            id: 2,
            kind: StreamKind::Audio,
            title: title.map(str::to_owned),
            language: language.map(str::to_owned),
            codec: None,
            playing: false,
        }
    }

    #[test]
    fn a_track_is_named_by_what_the_file_says_of_it() {
        const CASES: &[(&str, Option<&str>, Option<&str>, &str)] = &[
            ("both", Some("Commentary"), Some("en"), "Commentary (en)"),
            ("title only", Some("Commentary"), None, "Commentary"),
            ("language only", None, Some("fr"), "fr"),
            ("nothing", None, None, "Track 2"),
        ];
        for (name, title, language, want) in CASES {
            assert_eq!(track(*title, *language).name(), *want, "{name}");
        }
    }
}
