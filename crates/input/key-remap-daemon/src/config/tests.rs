use key_remap_core::{ConfigLayer, Hold, Profile, RuleAction, Stroke, Trigger, resolve};

use super::*;

fn map(s: &str) -> RuleAction {
    RuleAction::Map(
        Stroke::parse(s).expect("test strokes are valid"),
        Hold::Restore,
    )
}

fn rule<'a>(p: &'a Profile, when: &str) -> Option<&'a RuleAction> {
    p.rules
        .get(&Trigger::parse(when).expect("test triggers are valid"))
}

/// What `capslock` writes in the default profile once `layer` is laid over the built-in mapping.
fn capslock_in(layer: ConfigLayer) -> Option<RuleAction> {
    let (defaults, _) = key_remap_core::builtin();
    let (profiles, _) = resolve(&[defaults, layer]);
    rule(profiles.default_profile(), "capslock").cloned()
}

mod trusted_files {
    use std::path::Path;

    use caller_identity_core::{FileTrust, Uid, Writer};
    use caller_identity_linux::FixtureProc;
    use key_remap_core::ConfigSource;
    use key_remap_devtest::Scratch;

    use super::super::load::system_through;
    use super::super::trusted::LAYER_FILE_CAP;
    use super::super::vendor::vendor_through;
    use super::*;

    const CAPS_RULE: &str =
        "[profile.default]\nrules = [{ when = \"capslock\", then = \"ctrl+space\" }]\n";

    /// One of the two machine-wide files: where it lives, how it is read and what its
    /// warnings are called. The same table of cases runs over each.
    struct Layer {
        source: ConfigSource,
        /// The folder the file is in, and the folder above that.
        dir: &'static str,
        parent: &'static str,
        read: fn(&FixtureProc, &Path) -> (ConfigLayer, Vec<key_remap_proto::Warning>),
    }

    const LAYERS: [Layer; 2] = [
        Layer {
            source: ConfigSource::Vendor,
            dir: "/usr/lib/bardo/key-remap",
            parent: "/usr/lib",
            read: vendor_through::<FixtureProc>,
        },
        Layer {
            source: ConfigSource::System,
            dir: "/etc/bardo/key-remap",
            parent: "/etc",
            read: system_through::<FixtureProc>,
        },
    ];

    impl Layer {
        fn file(&self) -> String {
            format!("{}/{FILE_NAME}", self.dir)
        }

        /// What this layer in `fs` comes to: whether its rule applies, and the problems.
        fn read(&self, fs: &FixtureProc) -> (Option<RuleAction>, Vec<String>) {
            let (layer, warnings) = (self.read)(fs, Path::new(self.dir));
            let problems = warnings.into_iter().map(|w| w.problem).collect();
            (capslock_in(layer), problems)
        }
    }

    #[test]
    fn a_layer_file_is_read_only_when_root_alone_can_change_it() {
        let applies = Some(map("ctrl+space"));
        let owner = Writer::Owner(Uid::new(1000));
        for layer in LAYERS {
            let (dir, parent, file) = (layer.dir, layer.parent, layer.file());
            let over_cap = vec![b'#'; usize::try_from(LAYER_FILE_CAP).unwrap() + 1];
            let cases: Vec<(&str, FixtureProc, Option<RuleAction>, &str)> = vec![
                (
                    "root's own file",
                    FixtureProc::new().file(&file, CAPS_RULE, FileTrust::RootOnly),
                    applies.clone(),
                    "",
                ),
                (
                    "a file its group can write",
                    FixtureProc::new().file(&file, CAPS_RULE, FileTrust::Writable(Writer::Group)),
                    None,
                    "not used",
                ),
                (
                    "a file anyone can write",
                    FixtureProc::new().file(&file, CAPS_RULE, FileTrust::Writable(Writer::Other)),
                    None,
                    "not used",
                ),
                (
                    "a file a person owns",
                    FixtureProc::new().file(&file, CAPS_RULE, FileTrust::Writable(owner)),
                    None,
                    "not used",
                ),
                (
                    "a folder a person owns",
                    FixtureProc::new()
                        .dir(dir, FileTrust::Writable(owner))
                        .file(&file, CAPS_RULE, FileTrust::RootOnly),
                    None,
                    "not used",
                ),
                (
                    "a folder above that anyone can write",
                    FixtureProc::new()
                        .dir(parent, FileTrust::Writable(Writer::Other))
                        .file(&file, CAPS_RULE, FileTrust::RootOnly),
                    None,
                    "not used",
                ),
                (
                    "a file that is not text",
                    FixtureProc::new().file(&file, vec![0xffu8, 0xfe], FileTrust::RootOnly),
                    None,
                    "not text",
                ),
                (
                    "a file over the cap",
                    FixtureProc::new().file(&file, over_cap, FileTrust::RootOnly),
                    None,
                    "cannot be read",
                ),
                (
                    "a file that is not a config",
                    FixtureProc::new().file(&file, "[profile\n", FileTrust::RootOnly),
                    None,
                    "not valid TOML",
                ),
            ];
            let name = layer.source.to_string();
            for (case, fs, want, problem) in cases {
                let (got, problems) = layer.read(&fs);
                assert_eq!(got, want, "{name}: {case}");
                match problem {
                    "" => assert_eq!(problems, Vec::<String>::new(), "{name}: {case}"),
                    text => {
                        assert_eq!(problems.len(), 1, "{name}: {case}: {problems:?}");
                        assert!(problems[0].contains(text), "{name}: {case}: {problems:?}");
                    }
                }
                if !problems.is_empty() {
                    assert!(
                        problems[0].starts_with(&name),
                        "{name}: {case}: {problems:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_missing_file_or_folder_is_no_layer_and_no_problem() {
        for layer in LAYERS {
            let trees = [
                ("no folder", FixtureProc::new()),
                (
                    "an empty folder",
                    FixtureProc::new().dir(layer.dir, FileTrust::RootOnly),
                ),
            ];
            for (case, fs) in trees {
                assert_eq!(layer.read(&fs), (None, vec![]), "{}: {case}", layer.source);
            }
        }
        let dir = Scratch::new("no-file");
        type Read = fn(&Path) -> (ConfigLayer, Vec<key_remap_proto::Warning>);
        let reads: [Read; 2] = [super::super::vendor, super::super::system];
        for read in reads {
            assert_eq!(
                read(&dir.path().join("not-there")),
                (ConfigLayer::default(), vec![])
            );
            assert_eq!(read(dir.path()), (ConfigLayer::default(), vec![]));
        }
    }

    #[test]
    fn a_file_on_the_real_system_in_a_folder_a_person_owns_is_refused() {
        // The seam for tests that point `$BARDO_KEY_REMAP_CONFIG_DIR` at a scratch folder: a machine file
        // written there is the person's, so it gives no layer. A test that needs one has to say so.
        let dir = Scratch::new("person-owned");
        std::fs::write(dir.path().join(FILE_NAME), CAPS_RULE).expect("scratch file is writable");
        let (layer, warnings) = super::super::system(dir.path());
        assert_eq!(layer, ConfigLayer::default());
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0]
                .problem
                .starts_with("machine settings: not used")
        );
    }
}
