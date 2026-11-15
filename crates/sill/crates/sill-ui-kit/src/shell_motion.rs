//! The motion the compositor is told about (`quire_shell_v1.set_motion`): the person's
//! `appearance.motion_level`, reduced also when the desktop asks for reduced motion, as quire's
//! own resolve reads it.

use ds::prelude::Motion;
use ds_settings::Environment;
use sill_model::desktop::Motion as ShellMotion;

/// The motion of `environment`, in the compositor's words.
pub fn shell_motion(environment: &Environment) -> ShellMotion {
    match (
        environment.settings.appearance.motion_level,
        environment.system.reduced_motion,
    ) {
        (Motion::Standard, false) => ShellMotion::Full,
        (Motion::Reduced, _) | (Motion::Standard, true) => ShellMotion::Reduced,
    }
}

#[cfg(test)]
mod tests {
    use super::shell_motion;
    use ds::prelude::Motion;
    use ds_settings::Environment;
    use sill_model::desktop::Motion as ShellMotion;

    #[test]
    fn either_the_person_or_the_desktop_reduces_motion() {
        // (the person's level, the desktop's preference, what the compositor hears)
        let cases = [
            (Motion::Standard, false, ShellMotion::Full),
            (Motion::Reduced, false, ShellMotion::Reduced),
            (Motion::Standard, true, ShellMotion::Reduced),
            (Motion::Reduced, true, ShellMotion::Reduced),
        ];
        for (level, system, want) in cases {
            let mut environment = Environment::default();
            environment.settings.appearance.motion_level = level;
            environment.system.reduced_motion = system;
            assert_eq!(shell_motion(&environment), want, "{level:?} {system:?}");
        }
    }
}
