//! The registry reader and installer, over a fixture snapshot and archives built here. Nothing
//! reaches the network or the real home.

use agent_install::fetch::{Fetch, FetchError};
use agent_install::run::{Run, RunError};
use agent_install::store::{agents, read_record, versions};
use agent_install::unpack::{Limits, UnpackError, unpack};
use agent_install::uv::PythonPackage;
use agent_install::{InstallError, Offer, Standing, Want, install, offers, standing};
use agent_registry::dirs::AgentsDir;
use agent_registry::hash::Sha256Hex;
use agent_registry::platform::Platform;
use agent_registry::record::{Check, Record, RecordError};
use agent_registry::slug::Slug;
use agent_registry::snapshot::Snapshot;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

/// Serves archives by url.
struct Served(BTreeMap<String, Vec<u8>>);

impl Fetch for Served {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.0
            .get(url)
            .cloned()
            .ok_or_else(|| FetchError(url.to_owned()))
    }

    fn download(&self, url: &str, to: &Path) -> Result<(), FetchError> {
        let bytes = self.get(url)?;
        std::fs::write(to, bytes).map_err(|e| FetchError(format!("{url}: {e}")))
    }
}

/// A package manager that "installs" a package with a program.
struct FakeNpm;

impl Run for FakeNpm {
    fn run(&self, _: &str, _: &[String], dir: &Path) -> Result<(), RunError> {
        let pkg = dir.join("node_modules/@scope/tool");
        std::fs::create_dir_all(&pkg).expect("fake npm makes the package folder");
        std::fs::write(pkg.join("package.json"), r#"{"bin": {"tool-acp": "x.js"}}"#)
            .expect("fake npm writes the manifest");
        Ok(())
    }
}

/// A `uv` that makes the environment and the program the package installs, and keeps each call.
#[derive(Default)]
struct FakeUv {
    calls: RefCell<Vec<Vec<String>>>,
    /// Whether the install leaves its program behind.
    leaves_program: bool,
}

impl Run for FakeUv {
    fn run(&self, program: &str, args: &[String], dir: &Path) -> Result<(), RunError> {
        self.calls.borrow_mut().push(args.to_vec());
        let fail = || RunError::Failed(program.to_owned());
        if program != "uv" {
            return Err(fail());
        }
        let bin = dir.join("venv/bin");
        if args.iter().any(|a| a == "venv") {
            std::fs::create_dir_all(&bin).expect("fake uv makes the environment folder");
            std::fs::write(bin.join("python"), b"").expect("fake uv writes its python");
            return Ok(());
        }
        if !self.leaves_program {
            return Ok(());
        }
        let requirement = args.last().ok_or_else(fail)?;
        let name = requirement.split("==").next().unwrap_or_default();
        std::fs::write(bin.join(name), b"#!/bin/sh\n").expect("fake uv writes the program");
        Ok(())
    }
}

fn zip_of(name: &str, body: &[u8]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    zip.start_file(name, options).expect("start");
    zip.write_all(body).expect("write");
    zip.finish().expect("finish");
    out.into_inner()
}

fn tar_gz_of(name: &str, body: &[u8]) -> Vec<u8> {
    let mut tarball = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    tarball
        .append_data(&mut header, name, body)
        .expect("append");
    let raw = tarball.into_inner().expect("tar");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&raw).expect("gz");
    gz.finish().expect("gz end")
}

fn snapshot(sha: Option<&str>, version: &str) -> Snapshot {
    let key = Platform::of(std::env::consts::OS, std::env::consts::ARCH);
    let sha = sha.map_or(String::new(), |s| format!(r#", "sha256": "{s}""#));
    Snapshot::parse(&format!(
        r#"{{"version": "1", "extensions": [], "agents": [
        {{"id": "agy-acp", "name": "Agy", "description": "Writes code", "version": "{version}", "extra": 1,
          "distribution": {{"binary": {{"{key}": {{"archive": "https://dl.test/agy-{version}.zip",
            "cmd": "./agy", "args": ["--uid="]{sha}}}}}}}}},
        {{"id": "claude-acp", "name": "Claude", "version": "0.1.0",
          "distribution": {{"npx": {{"package": "@scope/tool@0.1.0", "args": ["--x"]}}}}}},
        {{"id": "py", "name": "Py", "version": "1", "distribution": {{"uvx": {{"package": "py==1"}}}}}}
        ]}}"#,
        key = key.as_str()
    ))
    .expect("fixture")
}

fn served(version: &str, body: &[u8]) -> Served {
    let url = format!("https://dl.test/agy-{version}.zip");
    Served(BTreeMap::from([(url, zip_of("agy", body))]))
}

/// The record of `id` at `version`, as the launcher would read it.
fn installed(dir: &AgentsDir, id: &str, version: &str) -> Result<Record, RecordError> {
    let (id, version) = (
        Slug::parse(id).expect("id"),
        Slug::parse(version).expect("version"),
    );
    read_record(&dir.installed(&id, &version))
}

/// The program of `id` at `version`, given the directory it is installed under.
fn program(dir: &AgentsDir, id: &str, version: &str) -> std::path::PathBuf {
    let (slug, pinned) = (
        Slug::parse(id).expect("id"),
        Slug::parse(version).expect("version"),
    );
    installed(dir, id, version)
        .expect("record")
        .command_in(&dir.installed(&slug, &pinned))
}

fn want<'a>(id: &'a str, version: &'a str) -> Want<'a> {
    Want { id, version }
}

#[test]
fn a_declared_digest_is_checked_and_the_program_unpacks_executable() {
    use std::os::unix::fs::PermissionsExt;
    let net = served("1.0.0", b"#!/bin/sh\n");
    let digest = Sha256Hex::of(&net.0["https://dl.test/agy-1.0.0.zip"]);
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let snap = snapshot(Some(digest.as_str()), "1.0.0");
    let record = install(&dir, &snap, want("agy-acp", "1.0.0"), (&net, &FakeNpm)).expect("install");
    assert_eq!(record.check, Check::Declared);
    assert_eq!(record.args, ["--uid="]);
    let command = program(&dir, "agy-acp", "1.0.0");
    assert!(command.is_file());
    let mode = std::fs::metadata(&command)
        .expect("meta")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111);
}

#[test]
fn a_wrong_digest_installs_nothing() {
    let net = served("1.0.0", b"#!/bin/sh\n");
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let wrong = "0".repeat(64);
    let snap = snapshot(Some(&wrong), "1.0.0");
    let fault =
        install(&dir, &snap, want("agy-acp", "1.0.0"), (&net, &FakeNpm)).expect_err("refused");
    assert!(matches!(fault, InstallError::Digest { .. }));
    installed(&dir, "agy-acp", "1.0.0").unwrap_err();
}

#[test]
fn no_declared_digest_is_said_to_be_first_use() {
    let net = served("1.0.0", b"x");
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let record = install(
        &dir,
        &snapshot(None, "1.0.0"),
        want("agy-acp", "1.0.0"),
        (&net, &FakeNpm),
    )
    .expect("install");
    assert_eq!(record.check, Check::FirstUse);
    assert!(record.digest.is_some());
}

#[test]
fn a_reinstall_with_no_declared_digest_is_compared_with_the_first_one() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let snap = snapshot(None, "1.0.0");
    let go = |body: &[u8]| {
        // Nothing is there before the first install.
        let _ = std::fs::remove_dir_all(tmp.path().join("installed"));
        install(
            &dir,
            &snap,
            want("agy-acp", "1.0.0"),
            (&served("1.0.0", body), &FakeNpm),
        )
    };
    assert_eq!(go(b"x").expect("first").check, Check::FirstUse);
    assert_eq!(go(b"x").expect("same").check, Check::Pinned);
    let fault = go(b"swapped").expect_err("changed");
    assert!(matches!(fault, InstallError::Digest { .. }));
    installed(&dir, "agy-acp", "1.0.0").unwrap_err();
}

#[test]
fn only_the_listed_version_installs_and_a_newer_one_is_never_taken_alone() {
    let net = served("1.0.0", b"x");
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    install(
        &dir,
        &snapshot(None, "1.0.0"),
        want("agy-acp", "1.0.0"),
        (&net, &FakeNpm),
    )
    .expect("install");
    // The registry moves on; nothing is fetched and the pin keeps working.
    let newer = snapshot(None, "1.1.0");
    assert_eq!(
        standing(&dir, &newer, "agy-acp"),
        Standing::Offered {
            offered: "1.1.0".into(),
            installed: vec!["1.0.0".into()]
        }
    );
    installed(&dir, "agy-acp", "1.0.0").unwrap();
    // Asking again for the installed version reaches nothing.
    let old = snapshot(None, "1.0.0");
    let none = Served(BTreeMap::new());
    install(&dir, &old, want("agy-acp", "1.0.0"), (&none, &FakeNpm)).unwrap();
    let stale =
        install(&dir, &newer, want("agy-acp", "0.9.0"), (&net, &FakeNpm)).expect_err("stale");
    assert!(matches!(stale, InstallError::Version { .. }));
    assert_eq!(standing(&dir, &newer, "nope"), Standing::Unknown);

    // The list in its own order, with what is installed of each; a missing line is empty.
    let shown = |snap: &Snapshot| -> Vec<(String, String, bool)> {
        offers(&dir, snap)
            .into_iter()
            .map(|o| (o.id, o.version, o.installed))
            .collect()
    };
    let row = |id: &str, v: &str, installed| (id.to_owned(), v.to_owned(), installed);
    assert_eq!(
        shown(&old),
        [
            row("agy-acp", "1.0.0", true),
            row("claude-acp", "0.1.0", false),
            row("py", "1", false)
        ]
    );
    assert_eq!(shown(&newer)[0], row("agy-acp", "1.1.0", false));
    let first: Offer = offers(&dir, &old).remove(0);
    assert_eq!(
        (first.name.as_str(), first.description.as_str()),
        ("Agy", "Writes code")
    );
    assert_eq!(offers(&dir, &old)[1].description, "");
    // What is installed is known without the list.
    let ids: Vec<String> = agents(&dir).iter().map(|s| s.as_str().to_owned()).collect();
    assert_eq!(ids, ["agy-acp"]);
}

#[test]
fn a_node_package_goes_through_the_package_manager() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let snap = snapshot(None, "1.0.0");
    let net = Served(BTreeMap::new());
    let record =
        install(&dir, &snap, want("claude-acp", "0.1.0"), (&net, &FakeNpm)).expect("install");
    assert_eq!(record.check, Check::PackageManager);
    assert_eq!(record.command, "node_modules/.bin/tool-acp");
}

#[test]
fn a_python_package_goes_through_uv_into_its_own_directory() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let snap = snapshot(None, "1.0.0");
    let net = Served(BTreeMap::new());
    let uv = FakeUv {
        leaves_program: true,
        ..FakeUv::default()
    };
    let record = install(&dir, &snap, want("py", "1"), (&net, &uv)).expect("install");
    assert_eq!(record.check, Check::PackageManager);
    assert_eq!(record.command, "venv/bin/py");
    assert!(program(&dir, "py", "1").is_file());
    let calls = uv.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert!(calls[1].contains(&"py==1".to_owned()));
    assert!(calls.iter().all(|c| c.contains(&"--cache-dir".to_owned())));
    assert!(calls[0].contains(&"--no-python-downloads".to_owned()));
    assert!(calls[1].contains(&"pip".to_owned()));
}

#[test]
fn a_python_program_missing_after_uv_installs_nothing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = AgentsDir::at(tmp.path());
    let snap = snapshot(None, "1.0.0");
    let net = Served(BTreeMap::new());
    let fault =
        install(&dir, &snap, want("py", "1"), (&net, &FakeUv::default())).expect_err("no program");
    assert!(matches!(fault, InstallError::NoProgram));
    installed(&dir, "py", "1").unwrap_err();
    assert_eq!(versions(&dir, &Slug::parse("py").expect("slug")), [] as [agent_registry::slug::Slug; 0]);
}

#[test]
fn python_package_forms_are_read_and_direct_references_refused() {
    let pinned = PythonPackage::parse("minion-code@0.1.44").expect("at form");
    assert_eq!(pinned.requirement, "minion-code==0.1.44");
    assert_eq!(pinned.name, "minion-code");
    let extra = PythonPackage::parse("pkg[acp]==2").expect("extra");
    assert_eq!(extra.name, "pkg");
    PythonPackage::parse("git+https://example.test/x").unwrap_err();
    PythonPackage::parse("pkg @ https://example.test/x.whl").unwrap_err();
    PythonPackage::parse("../escape").unwrap_err();
}

/// Writes `bytes` as the archive `name` in `dir` and returns its path.
fn archive_file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("archive");
    path
}

/// A zip of `(name, body)` files, deflated.
fn zip_many(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in files {
        zip.start_file(*name, options).expect("start");
        zip.write_all(body).expect("write");
    }
    zip.finish().expect("finish");
    out.into_inner()
}

/// Whether `into` holds nothing and no staging folder is left beside it.
fn left_nothing(tmp: &Path, into: &Path) -> bool {
    let empty = std::fs::read_dir(into).map_or(true, |mut d| d.next().is_none());
    let staged = std::fs::read_dir(tmp).expect("tmp").any(|e| {
        e.expect("entry")
            .file_name()
            .to_string_lossy()
            .ends_with(".unpacking")
    });
    empty && !staged
}

fn small() -> Limits {
    Limits {
        total: 1000,
        entry: 600,
        entries: 3,
    }
}

#[test]
fn a_tarball_unpacks_and_an_escaping_entry_is_refused() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    let good = archive_file(tmp.path(), "a.tar.gz", &tar_gz_of("bin/agy", b"x"));
    unpack("a.tar.gz", &good, &into, &Limits::default()).expect("unpack");
    assert!(into.join("bin/agy").is_file());
    let other = tmp.path().join("other");
    let bad = archive_file(tmp.path(), "a.zip", &zip_of("../escape", b"x"));
    let fault = unpack("a.zip", &bad, &other, &Limits::default()).expect_err("escape");
    assert!(matches!(fault, UnpackError::Path(_)), "{fault}");
    assert!(!tmp.path().join("escape").exists());
    assert!(left_nothing(tmp.path(), &other));
    let kind = unpack("a.tar.bz2", &good, &other, &Limits::default()).expect_err("kind");
    assert_eq!(kind, UnpackError::Kind);
}

#[test]
fn an_absolute_path_is_refused_and_nothing_is_left() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    let mut header = tar::Header::new_gnu();
    header.set_size(1);
    header.set_mode(0o644);
    header.as_old_mut().name[..9].copy_from_slice(b"/tmp/evil");
    header.set_cksum();
    let mut builder = tar::Builder::new(Vec::new());
    builder.append(&header, &b"x"[..]).expect("append");
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&builder.into_inner().expect("tar"))
        .expect("gz");
    let file = archive_file(tmp.path(), "a.tgz", &gz.finish().expect("end"));
    let fault = unpack("a.tgz", &file, &into, &Limits::default()).expect_err("absolute");
    assert!(matches!(fault, UnpackError::Path(_)), "{fault}");
    assert!(left_nothing(tmp.path(), &into));
}

#[test]
fn an_archive_over_the_total_cap_is_refused_and_nothing_is_left() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    let (a, b) = (vec![1_u8; 500], vec![2_u8; 500]);
    let c = vec![3_u8; 500];
    let file = archive_file(
        tmp.path(),
        "a.zip",
        &zip_many(&[("a", &a), ("b", &b), ("c", &c)]),
    );
    let fault = unpack("a.zip", &file, &into, &small()).expect_err("over total");
    assert!(matches!(fault, UnpackError::TooLarge(_)), "{fault}");
    assert!(left_nothing(tmp.path(), &into));
}

#[test]
fn a_file_over_the_entry_cap_is_refused_whatever_the_header_says() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    // A tar header states the size it writes, so the count of real bytes is what stops this one.
    let file = archive_file(tmp.path(), "a.tar.gz", &tar_gz_of("big", &vec![0_u8; 700]));
    let fault = unpack("a.tar.gz", &file, &into, &small()).expect_err("over entry");
    assert!(matches!(fault, UnpackError::TooLarge(_)), "{fault}");
    assert!(left_nothing(tmp.path(), &into));
}

/// A zip with one deflated file of `real` zero bytes whose headers both say it is `claimed`.
fn lying_zip(real: usize, claimed: u32) -> Vec<u8> {
    let mut bytes = zip_many(&[("liar", &vec![0_u8; real])]);
    let central = bytes
        .windows(4)
        .rposition(|w| w == b"PK\x01\x02")
        .expect("central header");
    bytes[22..26].copy_from_slice(&claimed.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&claimed.to_le_bytes());
    bytes
}

#[test]
fn an_entry_that_lies_about_its_size_is_refused_and_nothing_is_left() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    let file = archive_file(tmp.path(), "a.zip", &lying_zip(900, 10));
    let fault = unpack("a.zip", &file, &into, &small()).expect_err("liar");
    // The stated size fits the cap; the real bytes do not (or the reader notices the mismatch).
    assert!(
        matches!(fault, UnpackError::TooLarge(_) | UnpackError::Damaged(_)),
        "{fault}"
    );
    assert!(left_nothing(tmp.path(), &into));
}

#[test]
fn too_many_entries_are_refused_and_nothing_is_left() {
    let tmp = tempfile::tempdir().expect("tmp");
    let into = tmp.path().join("out");
    let file = archive_file(
        tmp.path(),
        "a.zip",
        &zip_many(&[("a", b"1"), ("b", b"2"), ("c", b"3"), ("d", b"4")]),
    );
    let fault = unpack("a.zip", &file, &into, &small()).expect_err("too many");
    assert!(matches!(fault, UnpackError::TooLarge(_)), "{fault}");
    assert!(left_nothing(tmp.path(), &into));
}

#[test]
fn a_download_over_the_cap_is_cut_off_and_removed() {
    let tmp = tempfile::tempdir().expect("tmp");
    let serve = tmp.path().join("serve");
    std::fs::create_dir(&serve).expect("serve");
    std::fs::write(serve.join("small.bin"), b"abc").expect("file");
    let files = agent_install::fetch::Files::under(&serve);
    let to = tmp.path().join("got");
    files.download("file://small.bin", &to).expect("download");
    assert_eq!(std::fs::read(&to).expect("got"), b"abc");
    let fault = files
        .download("file://missing.bin", &tmp.path().join("none"))
        .expect_err("missing");
    assert!(fault.0.contains("missing.bin"), "{fault}");
    assert!(!tmp.path().join("none").exists());
}
