//! The skills this repository ships validate against the manifests it ships.

use action_manifest::ValidManifest;
use agent_skills::{Library, Origin, Roots, discover, load_dir};
use agent_skills_disk::DiskFolder;
use std::path::PathBuf;

fn dist() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist/skills")
}

fn manifests() -> Vec<ValidManifest> {
    [
        include_str!("../../../manifests/org.bardo.Memory.toml"),
        include_str!("../../../manifests/org.bardo.Companion.toml"),
    ]
    .iter()
    .map(|t| toml::from_str(t).expect("manifest"))
    .collect()
}

#[test]
fn desktop_basics_validates_is_always_and_fits_a_kilobyte() {
    let skill =
        load_dir(&DiskFolder, &dist().join("desktop-basics"), Origin::Shipped).expect("validates");
    assert_eq!(skill.id.as_str(), "desktop-basics");
    assert!(skill.when.always);
    assert!(skill.body.len() <= 1024, "{} bytes", skill.body.len());
    let md = std::fs::read_to_string(dist().join("desktop-basics/SKILL.md")).expect("md");
    assert!(md.len() <= 1024, "the whole file is {} bytes", md.len());
    // It uses only the Companion's built-in actions, all registered and offered.
    assert!(
        skill
            .uses
            .iter()
            .all(|u| u.app.as_str() == "org.bardo.Companion")
    );
    let library = Library::check(vec![skill], &manifests());
    assert_eq!(library.hidden, [] as [agent_skills::Hidden; 0]);
    assert_eq!(library.offered(&manifests()).len(), 1);
}

#[test]
fn everything_under_dist_skills_loads() {
    let found = discover(&Roots::shipped_only(vec![dist()]), &DiskFolder);
    assert!(found.rejected.is_empty(), "{:?}", found.rejected);
    assert!(
        found
            .skills
            .iter()
            .any(|s| s.id.as_str() == "desktop-basics")
    );
}
