use model_provider::{ChoiceText, Extract, JsonText, Shape, ShapeFault};

/// A hand-written output type, as the five or six real ones will be.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict(String);

impl Extract for Verdict {
    fn shape() -> Shape {
        Shape::Choice(vec![ChoiceText("allow".into()), ChoiceText("deny".into())])
    }

    fn read(json: &JsonText) -> Result<Self, ShapeFault> {
        serde_json::from_str(json.as_str())
            .map(Verdict)
            .map_err(|_| ShapeFault::NotJson)
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
