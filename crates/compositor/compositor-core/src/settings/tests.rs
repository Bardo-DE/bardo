use super::*;

#[test]
fn outputs_in_the_skip_list_keep_their_own_curve_and_the_rest_warm() {
    use crate::ids::{OutputKey, OutputName};
    use crate::output::{EdidFacts, EdidSerial, GammaRamp, MinuteOfDay, NightLight, Schedule};
    let edid = |serial: EdidSerial| EdidFacts {
        make: "ACME".to_owned(),
        model: "M1".to_owned(),
        serial,
    };
    let output = |connector: &str, serial: EdidSerial| {
        let name = OutputName::parse(connector).expect("a connector name");
        let key = OutputKey::of(&edid(serial), &name);
        (name, key)
    };
    let by_name = output("DP-1", EdidSerial::Unknown);
    let by_key = output("HDMI-A-1", EdidSerial::Known("S1".to_owned()));
    let other = output("DP-2", EdidSerial::Unknown);
    let always = NightLightSettings::new(NightLight {
        schedule: Schedule::Always,
        ..NightLightSettings::defaults().light
    });
    let settings = always.skipping(vec![
        " DP-1 ".to_owned(),
        by_key.1.as_str().to_owned(),
        "connector:DP-3".to_owned(),
    ]);
    let noon = MinuteOfDay::wrapping(12 * 60);
    let ramp = |(name, key): &(OutputName, OutputKey)| settings.ramp_for(name, key, noon, 256);
    assert_eq!(ramp(&by_name), GammaRamp::Identity, "listed by connector");
    assert_eq!(ramp(&by_key), GammaRamp::Identity, "listed by its key");
    assert!(
        matches!(ramp(&other), GammaRamp::Table(_)),
        "an output not listed is warmed"
    );
    assert!(!settings.skips(&other.0, &other.1));
    let unlisted = NightLightSettings::defaults();
    assert!(
        unlisted.skip_outputs.is_empty(),
        "nothing is skipped by default"
    );
}
