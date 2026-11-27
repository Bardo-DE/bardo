//! PAM for real: the `live` suite. `BARDO_SHELL_PAM_TEST_USER` and `BARDO_SHELL_PAM_TEST_SECRET` must
//! name a throwaway account; the test fails, saying so, otherwise.

use std::path::Path;

use base_secret::SecretText;
use shell_model::auth::{Attempt, Outcome, UserName};
use shell_services::auth::pam;

/// PAM for real, only for a throwaway account named in the environment; never the user's own.
#[test]
fn pam_with_a_throwaway_account() {
    let (Ok(user), Ok(secret)) = (
        std::env::var("BARDO_SHELL_PAM_TEST_USER"),
        std::env::var("BARDO_SHELL_PAM_TEST_SECRET"),
    ) else {
        panic!(
            "BARDO_SHELL_PAM_TEST_USER and BARDO_SHELL_PAM_TEST_SECRET must name a throwaway account"
        );
    };
    let service = pam::service(Path::exists);
    let check = |text: &str| {
        pam::check(
            &service,
            &Attempt {
                user: UserName(user.clone()),
                secret: SecretText::new(text),
            },
        )
    };
    assert_eq!(check(&secret).outcome, Outcome::Accepted);
    assert_eq!(check("certainly not it").outcome, Outcome::Refused);
}
