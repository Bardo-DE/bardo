//! A title's matched characters drawn as `mark` runs: what a row shows where a palette's query
//! matched it (design/06-INTERACTIONS.md section 11.2).

use dioxus::prelude::*;

/// `text` with the characters at `marks` wrapped in `mark` runs.
pub(crate) fn marked(text: &str, marks: &[usize]) -> Element {
    let runs = runs(text, marks);
    rsx! {
        for (index , (run , hit)) in runs.into_iter().enumerate() {
            if hit {
                mark { key: "{index}", "{run}" }
            } else {
                Fragment { key: "{index}", "{run}" }
            }
        }
    }
}

/// `text` split into maximal runs of marked (`true`) and unmarked (`false`) characters.
pub(crate) fn runs(text: &str, marks: &[usize]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for (index, c) in text.chars().enumerate() {
        let hit = marks.contains(&index);
        match out.last_mut() {
            Some((run, last)) if *last == hit => run.push(c),
            _ => out.push((c.to_string(), hit)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::runs;

    #[test]
    fn marks_split_a_title_into_runs() {
        assert_eq!(
            runs("Re: UIDL", &[4, 5, 6, 7]),
            vec![
                ("Re: ".to_string(), false),
                ("UIDL".to_string(), true)
            ]
        );
    }
}
