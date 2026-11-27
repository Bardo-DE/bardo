use model_types::{ChoiceText, Extract, JsonText, Shape, ShapeError};

/// A hand-written output type, as the five or six real ones will be.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict(String);

impl Extract for Verdict {
    fn shape() -> Shape {
        Shape::Choice(vec![ChoiceText("allow".into()), ChoiceText("deny".into())])
    }

    #[expect(
        clippy::map_err_ignore,
        reason = "a shape fault never carries the reply: the model's text may be untrusted"
    )]
    fn read(json: &JsonText) -> Result<Self, ShapeError> {
        serde_json::from_str(json.as_str())
            .map(Verdict)
            .map_err(|_| ShapeError::NotJson)
    }
}

#[test]
fn a_verdict_reads_its_json() {
    assert_eq!(
        Verdict::read(&JsonText::new("\"allow\"").unwrap()),
        Ok(Verdict("allow".into()))
    );
    assert_eq!(Verdict::shape(), Verdict::shape());
}
