//! The Intelligence page: its sections in design/22's order, inferd's model picker over a fake
//! settings module in memory, and the page without inferd.

use crate::support::Sandbox;
use crate::support::drive::settle_on_threads;
use crate::support::drive::{click, scroll_into_view};
use crate::support::inferd::Inferd;
use account_capability::ModelId;
use account_core::AccountId;
use inference_core::{Declined, DeclinedBecause, InferEvent, ModelRef, Why};
use settings_app_client::ReasonFeed;
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use settings_dbus::Verdict;
use std::time::Duration;
use ui_harness::{Harness, Input};

mod absent;
mod picker;
mod sections;

fn intelligence() -> Opening {
    Opening {
        page: Some(PageSlug("intelligence".to_owned())),
        query: String::new(),
    }
}

/// A one-key schema of a program that has none of its own in the fixtures yet, written the way
/// design/22 section 3 names its keys.
fn stand_in(app: &str, file: &str, path: &str, section: &str) -> String {
    format!(
        "app = \"{app}\"\nfile = \"{file}\"\nversion = 1\n\n[[key]]\npath = \"{path}\"\n\
         default = true\nlabel = \"{path}\"\nhelp = \"\"\nsection = \"{section}\"\n\
         exposure = \"basic\"\n\n[key.kind]\nkind = \"toggle\"\n\n[key.page]\nkind = \"intelligence\"\n"
    )
}

/// Positions of each of `texts` in `html`, each searched for after the one before.
fn in_order(html: &str, texts: &[&str]) -> Vec<usize> {
    let mut from = 0;
    texts
        .iter()
        .map(|text| {
            let at = html[from..]
                .find(&format!(">{text}<"))
                .unwrap_or_else(|| panic!("{text:?} is not on the page after byte {from}"));
            from += at;
            from
        })
        .collect()
}

fn model(name: &str) -> ModelRef {
    ModelRef {
        account: AccountId::parse("local").expect("account"),
        model: ModelId::parse(name).expect("model"),
    }
}
