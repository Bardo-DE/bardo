//! A model layer turn as an inferd chat request: the inverse of what `inference-bridge` does inside inferd.
//!
//! The screenshots go inline (base64 in the frame), the sampling is the entry's or the provider's
//! as the caller chooses, and nothing here names a model: inferd routes by the tier.

use account_capability::Tier;
use account_core::DataClass;
use account_core::consent::Usage;
use inference_core as pi;
use model_provider as sp;
use model_vision_prep::MediaType;

/// Whose sampling a turn carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingFrom {
    /// The catalogue entry's (greedy for `--thinking off`): sent to the provider.
    Entry,
    /// None: the provider's own defaults (a model that takes no temperature).
    Provider,
}

/// Why a turn could not be written as a chat request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequestError {
    /// A vendor's own tool, which the wire cannot carry.
    #[error("not representable on the inference wire: {0}")]
    Wire(String),
    /// A shape of reply only a local engine can enforce.
    #[error("a reply shape only a local engine enforces")]
    Shape,
}

fn media(media: MediaType) -> &'static str {
    match media {
        MediaType::Png => "image/png",
        MediaType::Jpeg => "image/jpeg",
    }
}

fn part(one: &sp::Part) -> Result<pi::MessagePart, RequestError> {
    Ok(match one {
        sp::Part::Text(text) => pi::MessagePart::Text(text.clone()),
        sp::Part::Image(image) => pi::MessagePart::Image(pi::ImagePart {
            media_type: media(image.media).to_owned(),
            source: pi::ImageSource::Inline(pi::Base64Bytes(image.bytes.0.clone())),
        }),
        sp::Part::Thought { text, seal } => pi::MessagePart::Thought(pi::ThoughtPart {
            text: text.clone(),
            seal: seal.clone(),
        }),
        sp::Part::ToolCall(call) => pi::MessagePart::ToolCall(call.clone()),
        sp::Part::ToolResult(result) => pi::MessagePart::ToolResult(pi::ToolResultPart {
            id: pi::ToolCallId(result.id.0.clone()),
            status: result.status,
            parts: result
                .parts
                .iter()
                .map(part)
                .collect::<Result<_, RequestError>>()?,
        }),
    })
}

/// A tool result rides in a user message on the inference wire (it has no tool role).
fn role(role: sp::Role) -> pi::Role {
    match role {
        sp::Role::System => pi::Role::System,
        sp::Role::User | sp::Role::Tool => pi::Role::User,
        sp::Role::Assistant => pi::Role::Assistant,
    }
}

fn tool(spec: &sp::ToolSpec) -> Result<pi::ToolDecl, RequestError> {
    match spec {
        sp::ToolSpec::Function {
            name,
            description,
            parameters,
        } => Ok(pi::ToolDecl {
            name: name.clone(),
            description: description.clone(),
            params: parameters.clone(),
        }),
        sp::ToolSpec::Native(_) => Err(RequestError::Wire("a vendor's own tool".to_owned())),
    }
}

/// The chat request for `turn`, asked at `tier` for screen data.
///
/// # Errors
///
/// Returns a [`RequestError`] when the turn cannot be written as a chat request.
pub fn chat(
    turn: &sp::TurnRequest,
    tier: Tier,
    from: SamplingFrom,
) -> Result<pi::ChatRequest, RequestError> {
    let shape = match &turn.output {
        sp::OutputShape::Free => pi::ReplyShape::Text,
        _ => return Err(RequestError::Shape),
    };
    let messages: Vec<pi::ChatMessage> = turn
        .messages
        .iter()
        .map(|message| {
            Ok(pi::ChatMessage {
                role: role(message.role),
                parts: message
                    .parts
                    .iter()
                    .map(part)
                    .collect::<Result<_, RequestError>>()?,
            })
        })
        .collect::<Result<_, RequestError>>()?;
    let tools: Vec<pi::ToolDecl> = turn
        .tools
        .iter()
        .map(tool)
        .collect::<Result<_, RequestError>>()?;
    let control = pi::ChatControl::new()
        .with_tool_choice(turn.tool_choice.clone())
        .with_tool_calls(turn.tool_calls)
        .with_max_output(pi::Knob::Set(turn.limits.max_output))
        .with_reasoning(turn.reasoning)
        .with_sampling(match from {
            SamplingFrom::Entry => pi::Knob::Set(turn.sampling),
            SamplingFrom::Provider => pi::Knob::Off,
        })
        .with_stop(turn.limits.stop.clone());
    Ok(
        pi::ChatRequest::new(messages, tier, DataClass::Screen, Usage::Interactive)
            .with_shape(shape)
            .with_tools(tools)
            .with_control(control),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(output: sp::OutputShape) -> sp::TurnRequest {
        sp::TurnRequest {
            model: sp::ModelName("m".to_owned()),
            messages: vec![sp::Message {
                role: sp::Role::User,
                parts: vec![
                    sp::Part::Text("click the button".to_owned()),
                    sp::Part::Image(sp::ImageInput {
                        media: MediaType::Png,
                        bytes: sp::ImageBytes(vec![1, 2, 3]),
                        detail: sp::ImageDetail::Auto,
                    }),
                ],
            }],
            tools: vec![],
            tool_choice: sp::ToolChoice::Auto,
            tool_calls: sp::ToolParallelism::One,
            choice_scores: sp::ChoiceScores::Off,
            output,
            limits: sp::Limits {
                max_output: base_ids::Tokens(512),
                stop: vec![],
                deadline: None,
            },
            sampling: sp::Sampling {
                temperature: sp::Milli(0),
                top_p: sp::Knob::Off,
                top_k: sp::Knob::Off,
                min_p: sp::Knob::Off,
                repeat_penalty: sp::Knob::Off,
                seed: sp::Knob::Off,
            },
            reasoning: sp::Reasoning::Off,
            engine: sp::EngineExtras::None,
        }
    }

    #[test]
    fn a_screenshot_goes_inline_and_the_class_is_screen() {
        let request = chat(
            &turn(sp::OutputShape::Free),
            Tier::Balanced,
            SamplingFrom::Entry,
        )
        .expect("request");
        assert_eq!(request.class, DataClass::Screen);
        assert!(matches!(
            &request.messages[0].parts[1],
            pi::MessagePart::Image(pi::ImagePart {
                source: pi::ImageSource::Inline(_),
                ..
            })
        ));
        assert!(matches!(request.control.sampling, pi::Knob::Set(_)));
        assert_eq!(
            request.control.max_output,
            pi::Knob::Set(base_ids::Tokens(512))
        );
    }

    #[test]
    fn the_provider_can_choose_the_sampling() {
        let request = chat(
            &turn(sp::OutputShape::Free),
            Tier::Fast,
            SamplingFrom::Provider,
        )
        .expect("request");
        assert_eq!(request.control.sampling, pi::Knob::Off);
    }

    #[test]
    fn a_local_only_shape_is_refused() {
        let shaped = turn(sp::OutputShape::Regex("a".to_owned()));
        assert_eq!(
            chat(&shaped, Tier::Fast, SamplingFrom::Entry),
            Err(RequestError::Shape)
        );
    }
}
