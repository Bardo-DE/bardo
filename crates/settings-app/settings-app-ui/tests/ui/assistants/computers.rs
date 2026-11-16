//! Your computers: the lenders, adding a server, removing, and the guests asking to borrow.

use super::*;

#[test]
fn only_the_persons_own_computers_are_listed_and_there_is_no_address_to_type() {
    let c = computers("computers-list", vec![LAB_COMPUTER], vec![], vec![]);
    let text = group_text(&c.harness);
    for want in ["Your computers", "lab", "Qwen", "Ready", "Add\u{2026}"] {
        assert!(text.contains(want), "{want} in {text}");
    }
    for not in ["Acme One", "Local", "me@acme.example"] {
        assert!(
            !text.contains(not),
            "{not} is another kind of place: {text}"
        );
    }
    assert_eq!(c.harness.count(&format!("{GROUP} input")), 0);
    assert_eq!(
        c.fake.changes(),
        Vec::<String>::new(),
        "reading changes nothing"
    );
}

const SERVER_SHEET: &str = "[data-system=\"add-server\"]";

#[test]
fn a_server_is_added_from_its_sheet_and_shows_in_the_list_with_remove() {
    let mut c = computers_with_models("computers-server", vec![LAB_COMPUTER]);
    press(&mut c.harness, "Add a server");
    settle(&mut c.harness, "the sheet", |h| h.count(SERVER_SHEET) == 1);
    let sheet = c.harness.text_of(SERVER_SHEET).unwrap_or_default();
    assert!(
        sheet.contains("The server must answer on this computer."),
        "{sheet}"
    );
    settle(&mut c.harness, "the models", |h| {
        h.count("#server-model") == 1
    });
    type_into(&mut c.harness, "#server-name", "studio");
    type_into(&mut c.harness, "#server-port", "8080");
    click(&mut c.harness, "#server-add");
    settle(&mut c.harness, "the server in the list", |h| {
        h.count(SERVER_SHEET) == 0 && group_text(h).contains("studio")
    });
    assert_eq!(
        c.fake.changes(),
        ["add_server studio qwen port=8080 key=no"]
    );
    assert_eq!(
        c.harness
            .count(&format!("{GROUP} .ds-button[aria-label=\"Remove studio\"]")),
        1
    );
}

#[test]
fn a_guest_is_allowed_denied_and_forgotten() {
    let mut c = computers("computers-guests", vec![], vec![], vec![LAPTOP]);
    let asking = "Asking to use this computer's models";
    assert!(group_text(&c.harness).contains(asking));
    press(&mut c.harness, "Allow laptop");
    settle(&mut c.harness, "approved", |h| {
        group_text(h).contains("Can use this computer's models")
    });
    press(&mut c.harness, "Don\u{2019}t Allow laptop");
    settle(&mut c.harness, "denied", |h| {
        group_text(h).contains("Not allowed to use this computer's models")
    });
    press(&mut c.harness, "Forget laptop");
    settle(&mut c.harness, "forgotten", |h| {
        !group_text(h).contains("laptop")
    });
    assert_eq!(
        c.fake.changes(),
        ["answer n3 allow", "answer n3 deny", "forget n3"]
    );
}
