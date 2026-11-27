//! "On this computer": the language-model runtimes accountd lists (Ollama, llama.cpp, LM Studio),
//! each a row with its mark and state and a button to its account. A stopped one stays, greyed.
//! Under the group, the models the assistant runs itself with a Start or Stop each, a note while
//! it has not found llama.cpp, and the row that opens the engines' programs.

use crate::accounts::{AccountLink, OpenButton, use_listed};
use crate::copy;
use crate::copy::absence::Subject;
use crate::engines::{EnginePrograms, LocalEnginesNote};
use crate::running::RunningModels;
use crate::system::Absent;
use dioxus::prelude::*;
use settings_app_model::{Live, Runtime};
use ui_components::components::content::avatar::{AvatarSize, AvatarTone, person_hue};
use ui_components::components::content::label::{LabelRole, LabelStyle};
use ui_components::components::fields::field_row::{FieldGroup, FieldRow};
use ui_components::prelude::*;
use ui_components::root::pass_through::{DataAttr, DataName};

/// Where the list stands: the local runtimes accountd lists now, none when none runs.
pub(crate) type Found = Live<Vec<Runtime>>;

/// What accountd lists as runtimes, kept current.
pub(crate) fn use_runtimes() -> Memo<Found> {
    let listed = use_listed();
    use_memo(move || listed().map(Runtime::all_of))
}

/// The runtimes accountd lists, or why they are not there.
#[component]
pub fn LocalRuntimes() -> Element {
    let link = use_context::<AccountLink>();
    let found = use_runtimes();

    let group = match found() {
        Found::Looking => rsx! {},
        Found::Unavailable(absence) => rsx! {
            div { class: "runtimes", "data-runtimes": "unavailable",
                Absent {
                    title: crate::copy::common::LOCAL_MODELS,
                    words: copy::absence::absence(Subject::Runtimes, absence),
                }
            }
        },
        Found::Ready(runtimes) if runtimes.is_empty() => rsx! {
            div { class: "runtimes", "data-runtimes": "empty",
                Absent {
                    title: crate::copy::common::LOCAL_MODELS,
                    words: crate::copy::picker::NOTHING_RUNNING_HERE,
                }
            }
        },
        Found::Ready(runtimes) => rsx! {
            div { class: "runtimes", "data-runtimes": "list",
                FieldGroup { title: Some(crate::copy::picker::MODELS_ON_THIS_COMPUTER.to_owned()),
                    for runtime in runtimes {
                        RuntimeRow {
                            key: "{runtime.key.0}",
                            runtime: runtime.clone(),
                            onopen: move |()| link.open(runtime.key.clone()),
                        }
                    }
                }
            }
        },
    };
    rsx! {
        {group}
        RunningModels {}
        LocalEnginesNote {}
        EnginePrograms {}
    }
}

/// One runtime: its round mark, how it stands, and the way to its account. A stopped runtime has
/// a muted mark and a quiet state, and still opens.
#[component]
fn RuntimeRow(runtime: Runtime, onopen: EventHandler<()>) -> Element {
    let running = runtime.is_running();
    let (muting, role, state) = if running {
        (false, LabelRole::Secondary, "running")
    } else {
        (true, LabelRole::Tertiary, "offline")
    };
    let initial = runtime
        .label
        .chars()
        .next()
        .unwrap_or('?')
        .to_ascii_uppercase();
    let common = Common {
        data: DataName::parse("runtime").map_or_default(|name| vec![DataAttr::new(name, state)]),
        ..Common::default()
    };
    rsx! {
        FieldRow {
            label: TextLine::from(runtime.label.clone()),
            common,
            Avatar {
                initial,
                size: AvatarSize::Size28,
                tone: AvatarTone::Person(person_hue(&runtime.label)),
                muting,
            }
            Label { text: copy::first_run::runtime_state(&runtime), role, style: LabelStyle::Footnote }
            OpenButton { label: crate::copy::common::MANAGE, subject: runtime.label.clone(), onpress: move |()| onopen.call(()) }
        }
    }
}
