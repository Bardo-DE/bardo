//! `cargo xtask check layers`: the rules in `layers.toml` at the repository root.
//!
//! The file says which domain folder each crate sits in (and so which crates it may depend on),
//! which crates are programs, and what a crate may not reach (`[[reach]]`), what its direct
//! workspace dependencies are (`[[edges]]`), which dependencies it may not name (`[[direct]]`),
//! which packages the lock file may not hold (`[[absent]]`), how big a tree may grow (`[[budget]]`),
//! the order inside a group of crates (`[[tiers]]`) and what its source may not contain
//! (`[[source]]`). Parsing and rule evaluation are functions over plain data; cargo and the file
//! system are touched only in [`run`] and the few helpers next to it.
//!
//! Output is `path:line: layers: message`. A finding about a rule points at the rule's table header
//! in `layers.toml`; a finding about a crate points at its manifest; a finding about source points
//! at the source line.
//!
//! # Rules
//!
//! `[[reach]]`: `crate` (or `crates`, a glob over member names, with `except = [names]`) with the
//! selected `features` must not reach any package in `forbid` through any path. `features` is
//! `"default"` (default), `"all"`, `"none"` or a list of names. `build` (default true) also follows
//! build dependencies; `dev` (default false) also follows dev-dependencies; `prune` names crates
//! whose own dependencies the walk does not enter. `forbid-features` lists `crate/feature` pairs
//! that must not be enabled anywhere in the tree (`forbid` may then be empty). One `cargo tree` run
//! serves every reach and budget rule with the same crate, features, kinds and prune list.
//!
//! `[[edges]]`: `exactly` is the list of the crate's direct dependencies (normal; plus build when
//! `build = true`) of the selected `features`. `scope` is `"workspace"` (default; only dependencies
//! on workspace members are compared) or `"all"`.
//!
//! `[[direct]]`: the crate (or every member matching `crates`, minus `except`) names none of
//! `forbid` as a declared dependency under the selected `features` and `build` (default true),
//! except names in `allow`. Names in `forbid` and `allow` end in `*` for a prefix match.
//!
//! `[[absent]]`: no package named in `names` (prefix globs allowed) appears in the root
//! `Cargo.lock`, dev-dependencies included.
//!
//! `[[budget]]`: `cargo tree -p crate` (`features`, `build`, `prune` as in `[[reach]]`) lists at most
//! `max` packages, counting each name and version once.
//!
//! `[[tiers]]`: `tiers` is a list of lists of crates, lowest first. A listed crate depends directly
//! (all features, normal and build) only on crates of its own tier, a lower tier, or crates the
//! rule does not list. A listed crate that is not a member is a finding.
//!
//! `[[source]]`: no line matching `pattern` (a Rust regex, inline flags allowed) in the files under
//! `path` (a file, a directory or a glob). By default only `.rs` files are read, test files and
//! folders (`tests.rs`, `*_tests.rs`, `testkit.rs`, `tests/`, `benches/`, `examples/`) are skipped,
//! comment lines are skipped and reading stops at the first `#[cfg(test)]` line. `except` lists
//! globs over paths relative to the root; a match on a directory skips everything under it.
//! `comments = true` also matches comment lines. `tests = true` also reads test files and folders
//! and does not stop at `#[cfg(test)]`. `files = "all"` reads every file that reads as UTF-8 text
//! (never under `.git` or `target`) and also reports a file or directory whose relative path
//! matches the pattern.
//!
//! Edge features (`[[edges]]`, `[[direct]]`, `[[tiers]]`) are read from the declared dependencies,
//! not a full resolve: a
//! non-optional normal dependency is always present; an optional one is present when a selected
//! feature names it, enables it with `dep:x`, or enables a feature of it with `x/feat` (`x?/feat`
//! does not enable it). Selected features are followed transitively. `default` starts from the
//! `default` feature, `all` selects every feature and optional dependency, `none` nothing, and a
//! list exactly those names. Target-specific dependencies count as declared.
//!
//! `[[edges]]` compares `exactly` against the crate's selected direct dependencies of the chosen
//! kinds. A name in a `forbid` list ending in `*` is a prefix match (`tokio*` matches `tokio-util`).
//! A `[[source]]` `path` is a single file, a directory (walked) or a glob (segments with `*` and
//! `?`).
//!
//! Members outside `crates/` (such as `xtask`) need a row like every member, but are exempt from the
//! domain-folder rule and the layer order.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cargo_metadata::{DependencyKind, MetadataCommand};
use regex::Regex;
use serde::Deserialize;

use super::finish;
use crate::workspace::Workspace;

/// The crate kinds a row may name.
const KINDS: [&str; 9] = [
    "types",
    "core",
    "proto",
    "io",
    "client",
    "service",
    "ui",
    "test-support",
    "unclassified",
];

/// A crate name ending in one of these is a program, not a library.
const PROGRAM_SUFFIXES: [&str; 14] = [
    "-daemon", "-app", "-cli", "-bin", "-eval", "-ui", "-testkit", "-fake", "-accept", "-gallery",
    "-bench", "-demo", "-tool", "-harness",
];

// ---------------------------------------------------------------------------------------------
// The file
// ---------------------------------------------------------------------------------------------

/// Which features of a crate a rule looks at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[serde(try_from = "RawSelection")]
enum Selection {
    #[default]
    Default,
    All,
    None,
    List(Vec<String>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawSelection {
    Word(String),
    List(Vec<String>),
}

impl TryFrom<RawSelection> for Selection {
    type Error = String;

    fn try_from(raw: RawSelection) -> Result<Self, String> {
        match raw {
            RawSelection::Word(word) => match word.as_str() {
                "default" => Ok(Self::Default),
                "all" => Ok(Self::All),
                "none" => Ok(Self::None),
                other => Err(format!(
                    "features `{other}`: expected \"default\", \"all\", \"none\" or a list"
                )),
            },
            RawSelection::List(list) => Ok(Self::List(list)),
        }
    }
}

impl Selection {
    /// The `cargo tree` flags that select these features.
    fn tree_flags(&self) -> Vec<String> {
        match self {
            Self::Default => Vec::new(),
            Self::All => vec!["--all-features".to_owned()],
            Self::None => vec!["--no-default-features".to_owned()],
            Self::List(names) if names.is_empty() => vec!["--no-default-features".to_owned()],
            Self::List(names) => vec![
                "--no-default-features".to_owned(),
                "--features".to_owned(),
                names.join(","),
            ],
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layers {
    domains: Vec<String>,
    #[serde(default)]
    kind: BTreeMap<String, KindRules>,
    #[serde(default, rename = "crate")]
    krate: BTreeMap<String, Row>,
    #[serde(default)]
    reach: Vec<Reach>,
    #[serde(default)]
    edges: Vec<Edges>,
    #[serde(default)]
    source: Vec<Source>,
    #[serde(default)]
    direct: Vec<Direct>,
    #[serde(default)]
    absent: Vec<Absent>,
    #[serde(default)]
    budget: Vec<Budget>,
    #[serde(default)]
    tiers: Vec<TierOrder>,
    #[serde(default)]
    known: Vec<Known>,
}

/// A finding the workspace has today and a later change removes: it is reported but does not
/// fail the check, and an entry that matches no finding fails it (so the list only shrinks).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    /// Text each finding contains (its path, a space and its message), such as
    /// `agent-install reaches reqwest:`; every one must match a finding.
    findings: Vec<String>,
    /// Why it is there and what removes it.
    why: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct KindRules {
    #[serde(default)]
    forbid: Vec<String>,
    #[serde(default, rename = "forbid-src")]
    forbid_src: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    kind: Option<String>,
    #[serde(default)]
    program: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reach {
    #[serde(rename = "crate")]
    krate: Option<String>,
    crates: Option<String>,
    #[serde(default)]
    except: Vec<String>,
    #[serde(default)]
    features: Selection,
    #[serde(default = "yes")]
    build: bool,
    /// Crates whose own dependencies the walk does not enter (`cargo tree --prune`).
    #[serde(default)]
    prune: Vec<String>,
    /// Also follow dev-dependencies.
    #[serde(default)]
    dev: bool,
    #[serde(default)]
    forbid: Vec<String>,
    /// `crate/feature` pairs that must not be enabled anywhere in the tree.
    #[serde(default, rename = "forbid-features")]
    forbid_features: Vec<String>,
    why: String,
}

/// What a crate (or every member matching `crates`) names directly as a dependency.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Direct {
    #[serde(rename = "crate")]
    krate: Option<String>,
    crates: Option<String>,
    #[serde(default)]
    except: Vec<String>,
    #[serde(default)]
    features: Selection,
    #[serde(default = "yes")]
    build: bool,
    forbid: Vec<String>,
    /// Names exempt from `forbid`.
    #[serde(default)]
    allow: Vec<String>,
    why: String,
}

/// Packages that appear nowhere in `Cargo.lock`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Absent {
    names: Vec<String>,
    why: String,
}

/// A limit on the number of distinct packages in a crate's tree.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    #[serde(rename = "crate")]
    krate: String,
    #[serde(default)]
    features: Selection,
    #[serde(default = "yes")]
    build: bool,
    #[serde(default)]
    prune: Vec<String>,
    max: usize,
    why: String,
}

/// An order inside a group of crates.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TierOrder {
    name: String,
    tiers: Vec<Vec<String>>,
    why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edges {
    #[serde(rename = "crate")]
    krate: String,
    #[serde(default)]
    features: Selection,
    #[serde(default)]
    build: bool,
    /// `"workspace"` (default) compares only dependencies on workspace members; `"all"` compares
    /// outside crates too.
    #[serde(default)]
    scope: Scope,
    exactly: Vec<String>,
    why: String,
}

/// Which dependencies an `[[edges]]` rule compares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Scope {
    /// Dependencies on workspace members only.
    #[default]
    Workspace,
    /// Every direct dependency.
    All,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    pattern: String,
    /// Files or directories (globs over paths relative to the root) the rule skips.
    #[serde(default)]
    except: Vec<String>,
    /// Also match comment lines.
    #[serde(default)]
    comments: bool,
    /// Also read test files and code after `#[cfg(test)]`.
    #[serde(default)]
    tests: bool,
    #[serde(default)]
    files: Files,
    why: String,
}

/// Which files a `[[source]]` rule reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Files {
    /// Rust source files.
    #[default]
    Rs,
    /// Every file that reads as text; paths are matched too.
    All,
}

impl Source {
    fn scan(&self) -> ScanOptions {
        ScanOptions {
            except: self.except.clone(),
            comments: self.comments,
            tests: self.tests,
            all_files: self.files == Files::All,
        }
    }
}

impl Layers {
    /// Parses and validates the file text.
    fn parse(text: &str) -> Result<Self> {
        let layers: Self = toml::from_str(text).context("layers.toml does not parse")?;
        for (name, row) in &layers.krate {
            if let Some(kind) = &row.kind
                && !KINDS.contains(&kind.as_str())
            {
                bail!("layers.toml: [crate.{name}] has unknown kind `{kind}`");
            }
        }
        for name in layers.kind.keys() {
            if !KINDS.contains(&name.as_str()) {
                bail!("layers.toml: [kind.{name}] is not a kind");
            }
        }
        for reach in &layers.reach {
            if reach.krate.is_some() == reach.crates.is_some() {
                bail!("layers.toml: a [[reach]] has exactly one of `crate` and `crates`");
            }
            if reach.krate.is_some() && !reach.except.is_empty() {
                bail!("layers.toml: `except` goes with `crates`, not `crate`");
            }
        }
        for direct in &layers.direct {
            if direct.krate.is_some() == direct.crates.is_some() {
                bail!("layers.toml: a [[direct]] has exactly one of `crate` and `crates`");
            }
            if direct.krate.is_some() && !direct.except.is_empty() {
                bail!("layers.toml: `except` goes with `crates`, not `crate`");
            }
        }
        for tiers in &layers.tiers {
            let mut seen = BTreeSet::new();
            for name in tiers.tiers.iter().flatten() {
                if !seen.insert(name) {
                    bail!("layers.toml: [[tiers]] {} lists `{name}` twice", tiers.name);
                }
            }
        }
        for source in &layers.source {
            Regex::new(&source.pattern)
                .with_context(|| format!("layers.toml: [[source]] pattern `{}`", source.pattern))?;
        }
        for (name, rules) in &layers.kind {
            for pattern in &rules.forbid_src {
                Regex::new(pattern).with_context(|| {
                    format!("layers.toml: [kind.{name}] forbid-src `{pattern}`")
                })?;
            }
        }
        Ok(layers)
    }
}

// ---------------------------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------------------------

/// One problem, reported as `path:line: layers: message`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    path: String,
    line: usize,
    message: String,
}

impl Finding {
    fn new(path: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: layers: {}", self.path, self.line, self.message)
    }
}

const FILE: &str = "layers.toml";

/// The 1-based lines of each table header equal to `header` (`[[reach]]`), in file order.
fn header_lines(text: &str, header: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.trim() == header)
        .map(|(index, _)| index + 1)
        .collect()
}

/// The 1-based line of `[<table>.<name>]` (the name may be quoted), or 1.
fn table_line(text: &str, table: &str, name: &str) -> usize {
    let plain = format!("[{table}.{name}]");
    let quoted = format!("[{table}.\"{name}\"]");
    text.lines()
        .position(|line| {
            let line = line.trim();
            line == plain || line == quoted
        })
        .map_or(1, |index| index + 1)
}

// ---------------------------------------------------------------------------------------------
// Workspace data
// ---------------------------------------------------------------------------------------------

/// One declared dependency.
#[derive(Debug, Clone)]
struct Dep {
    /// The package name.
    name: String,
    /// The key the manifest gives it (differs from `name` when renamed).
    key: String,
    kind: DependencyKind,
    optional: bool,
}

/// A workspace member as far as the rules are concerned.
#[derive(Debug, Clone)]
struct Member {
    name: String,
    /// The manifest, relative to the root.
    manifest: PathBuf,
    /// The manifest directory, relative to the root.
    dir: PathBuf,
    has_bin: bool,
    /// Whether it has a library target another crate could depend on.
    has_lib: bool,
    deps: Vec<Dep>,
    features: BTreeMap<String, Vec<String>>,
}

/// The `<domain>` of `crates/<domain>/...`.
fn domain_of(dir: &Path) -> Option<String> {
    let mut parts = dir.components();
    if parts.next()?.as_os_str() != "crates" {
        return None;
    }
    Some(parts.next()?.as_os_str().to_str()?.to_owned())
}

/// Whether a package is a program by its shape: a `bin` target, a telling name, or an `examples` folder.
fn is_program(name: &str, dir: &Path, has_bin: bool) -> bool {
    has_bin
        || PROGRAM_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || name.contains("-eval-")
        || dir.components().any(|part| part.as_os_str() == "examples")
}

/// Whether `member` is a program: its row says so, or [`is_program`] does.
fn program_of(layers: &Layers, member: &Member) -> bool {
    layers
        .krate
        .get(&member.name)
        .is_some_and(|row| row.program)
        || is_program(&member.name, &member.dir, member.has_bin)
}

/// A glob over one name or path segment: `*` is any run of characters, `?` any one.
fn glob_match(pattern: &str, text: &str) -> bool {
    fn go(pattern: &[char], text: &[char]) -> bool {
        match pattern.split_first() {
            None => text.is_empty(),
            Some(('*', rest)) => (0..=text.len()).any(|skip| go(rest, &text[skip..])),
            Some(('?', rest)) => !text.is_empty() && go(rest, &text[1..]),
            Some((c, rest)) => text.first() == Some(c) && go(rest, &text[1..]),
        }
    }
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    go(&pattern, &text)
}

// ---------------------------------------------------------------------------------------------
// Rules over plain data
// ---------------------------------------------------------------------------------------------

/// Every member has a row, every row names a member, every member under `crates/` is in a domain folder
/// (members outside `crates/` are exempt from that last rule).
fn structure_findings(layers: &Layers, members: &[Member], text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let names: BTreeSet<&str> = members.iter().map(|m| m.name.as_str()).collect();
    for member in members {
        if !layers.krate.contains_key(&member.name) {
            findings.push(Finding::new(
                member.manifest.display().to_string(),
                1,
                format!(
                    "{} has no [crate.{}] row in {FILE}",
                    member.name, member.name
                ),
            ));
        }
    }
    for name in layers.krate.keys() {
        if !names.contains(name.as_str()) {
            findings.push(Finding::new(
                FILE,
                table_line(text, "crate", name),
                format!("[crate.{name}] names no workspace member"),
            ));
        }
    }
    for member in members {
        if member.dir.starts_with("crates") && domain_of(&member.dir).is_none() {
            findings.push(Finding::new(
                member.manifest.display().to_string(),
                1,
                format!("{} is not in a domain folder crates/<domain>/", member.name),
            ));
        }
    }
    findings
}

/// A workspace package under `crates/`, as far as the layer order is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Package {
    name: String,
    manifest: PathBuf,
    domain: String,
    program: bool,
}

/// The position of `domain` in `domains`.
fn rank(domains: &[String], domain: &str) -> Option<usize> {
    domains.iter().position(|known| known == domain)
}

/// One finding for each library that depends on a library of a higher domain, and one for each
/// package in a domain `domains` does not name. `edges` are `(dependent, dependency)` normal dependencies.
fn violations(
    domains: &[String],
    packages: &[Package],
    edges: &[(String, String)],
) -> Vec<Finding> {
    let find = |name: &str| packages.iter().find(|package| package.name == name);
    let mut findings = Vec::new();
    for package in packages {
        if rank(domains, &package.domain).is_none() {
            findings.push(Finding::new(
                package.manifest.display().to_string(),
                1,
                format!(
                    "{} is in domain `{}`, which `domains` does not name",
                    package.name, package.domain
                ),
            ));
        }
    }
    for (dependent, dependency) in edges {
        let (Some(from), Some(to)) = (find(dependent), find(dependency)) else {
            continue;
        };
        if from.program || to.program {
            continue;
        }
        if let (Some(from_rank), Some(to_rank)) =
            (rank(domains, &from.domain), rank(domains, &to.domain))
            && to_rank > from_rank
        {
            findings.push(Finding::new(
                from.manifest.display().to_string(),
                1,
                format!(
                    "layer order: {} ({}) -> {} ({})",
                    from.name, from.domain, to.name, to.domain
                ),
            ));
        }
    }
    findings
}

/// One finding for each normal or build dependency on a program. `edges` are `(dependent, dependency)`.
fn leaf_findings(
    programs: &BTreeSet<String>,
    manifests: &BTreeMap<String, PathBuf>,
    edges: &[(String, String)],
) -> Vec<Finding> {
    edges
        .iter()
        .filter(|(_, dependency)| programs.contains(dependency))
        .filter_map(|(dependent, dependency)| {
            let manifest = manifests.get(dependent)?;
            Some(Finding::new(
                manifest.display().to_string(),
                1,
                format!("{dependent} depends on the program {dependency}; programs are leaves"),
            ))
        })
        .collect()
}

/// A forbidden crate (or crate/feature pair) and why.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Forbid {
    name: String,
    why: String,
    /// The `layers.toml` line of the rule.
    line: usize,
}

/// A `[[budget]]` limit on the number of packages in a tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Limit {
    max: usize,
    why: String,
    /// The `layers.toml` line of the rule.
    line: usize,
}

/// What one `cargo tree` run is checked against.
#[derive(Debug, Default, PartialEq, Eq)]
struct JobRules {
    /// Package names the tree must not contain.
    forbids: Vec<Forbid>,
    /// `crate/feature` pairs that must not be enabled anywhere in the tree.
    features: Vec<Forbid>,
    /// Limits on the number of distinct packages.
    limits: Vec<Limit>,
}

/// A `cargo tree` to run: one crate, one feature selection, one edge-kind choice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TreeJob {
    krate: String,
    features: Selection,
    build: bool,
    dev: bool,
    prune: Vec<String>,
    /// The host's platform only: a budget counts what this machine builds, as its number was set;
    /// the other rules read every platform's dependencies.
    host: bool,
}

impl TreeJob {
    fn args(&self) -> Vec<String> {
        let mut kinds = vec!["normal"];
        if self.build {
            kinds.push("build");
        }
        if self.dev {
            kinds.push("dev");
        }
        let mut args = vec![
            "tree".to_owned(),
            "-p".to_owned(),
            self.krate.clone(),
            "-e".to_owned(),
            kinds.join(","),
            "--prefix".to_owned(),
            "none".to_owned(),
        ];
        if !self.host {
            args.extend(["--target".to_owned(), "all".to_owned()]);
        }
        args.extend(["-f".to_owned(), "{p} {f}".to_owned()]);
        args.extend(self.features.tree_flags());
        for name in &self.prune {
            args.extend(["--prune".to_owned(), name.clone()]);
        }
        args
    }
}

/// The members a rule applies to: its `crate` (a finding when it is not a member) or every member
/// matching its `crates` glob except the `except` names.
fn select_members<'a>(
    table: &str,
    krate: Option<&str>,
    crates: Option<&str>,
    except: &[String],
    member_names: &[&'a str],
    line: usize,
    findings: &mut Vec<Finding>,
) -> Vec<&'a str> {
    match (krate, crates) {
        (Some(name), _) => {
            if let Some(found) = member_names.iter().copied().find(|m| *m == name) {
                vec![found]
            } else {
                findings.push(Finding::new(
                    FILE,
                    line,
                    format!("[[{table}]] crate `{name}` is not a workspace member"),
                ));
                Vec::new()
            }
        }
        (None, Some(glob)) => member_names
            .iter()
            .copied()
            .filter(|name| glob_match(glob, name))
            .filter(|name| !except.iter().any(|skip| skip == name))
            .collect(),
        (None, None) => Vec::new(),
    }
}

/// Reach and budget rules, as jobs with what each is checked against. A rule whose crate is not a
/// member is a finding.
fn reach_jobs(
    layers: &Layers,
    member_names: &[&str],
    text: &str,
) -> (BTreeMap<TreeJob, JobRules>, Vec<Finding>) {
    let mut jobs: BTreeMap<TreeJob, JobRules> = BTreeMap::new();
    let mut findings = Vec::new();
    let headers = header_lines(text, "[[reach]]");
    for (index, rule) in layers.reach.iter().enumerate() {
        let line = headers.get(index).copied().unwrap_or(1);
        let targets = select_members(
            "reach",
            rule.krate.as_deref(),
            rule.crates.as_deref(),
            &rule.except,
            member_names,
            line,
            &mut findings,
        );
        for target in targets {
            let job = TreeJob {
                krate: target.to_owned(),
                features: rule.features.clone(),
                build: rule.build,
                dev: rule.dev,
                prune: rule.prune.clone(),
                host: false,
            };
            let entry = jobs.entry(job).or_default();
            let forbid = |name: &String| Forbid {
                name: name.clone(),
                why: rule.why.clone(),
                line,
            };
            entry.forbids.extend(rule.forbid.iter().map(forbid));
            entry
                .features
                .extend(rule.forbid_features.iter().map(forbid));
        }
    }
    let budget_headers = header_lines(text, "[[budget]]");
    for (index, rule) in layers.budget.iter().enumerate() {
        let line = budget_headers.get(index).copied().unwrap_or(1);
        let targets = select_members(
            "budget",
            Some(rule.krate.as_str()),
            None,
            &[],
            member_names,
            line,
            &mut findings,
        );
        for target in targets {
            let job = TreeJob {
                krate: target.to_owned(),
                features: rule.features.clone(),
                build: rule.build,
                dev: false,
                prune: rule.prune.clone(),
                host: true,
            };
            jobs.entry(job).or_default().limits.push(Limit {
                max: rule.max,
                why: rule.why.clone(),
                line,
            });
        }
    }
    for (member_kind, rules) in kind_rules(layers) {
        for (member, _) in member_kind {
            let line = table_line(text, "crate", &member);
            if !member_names.contains(&member.as_str()) || rules.forbid.is_empty() {
                continue;
            }
            let job = TreeJob {
                krate: member,
                features: Selection::Default,
                build: true,
                dev: false,
                prune: Vec::new(),
                host: false,
            };
            let why = format!("crates of this kind may not reach it: {}", rules.name);
            jobs.entry(job)
                .or_default()
                .forbids
                .extend(rules.forbid.iter().map(|name| Forbid {
                    name: name.clone(),
                    why: why.clone(),
                    line,
                }));
        }
    }
    (jobs, findings)
}

/// The rules of a kind, with the crates (and their row lines) that have it.
struct KindUse<'a> {
    name: &'a str,
    forbid: &'a [String],
    forbid_src: &'a [String],
}

/// For each kind table, the crates whose row names that kind (never `unclassified`).
fn kind_rules(layers: &Layers) -> Vec<(Vec<(String, usize)>, KindUse<'_>)> {
    layers
        .kind
        .iter()
        .filter(|(name, _)| name.as_str() != "unclassified")
        .map(|(name, rules)| {
            let crates = layers
                .krate
                .iter()
                .filter(|(_, row)| row.kind.as_deref() == Some(name.as_str()))
                .map(|(krate, _)| (krate.clone(), 1))
                .collect();
            (
                crates,
                KindUse {
                    name,
                    forbid: &rules.forbid,
                    forbid_src: &rules.forbid_src,
                },
            )
        })
        .collect()
}

/// One package line of `cargo tree --prefix none -f '{p} {f}'`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeLine {
    name: String,
    version: String,
    /// The features enabled on this package in the tree.
    features: Vec<String>,
}

/// The packages in `cargo tree --prefix none -f '{p} {f}'` output. A line is
/// `name vVERSION [(path)] [(proc-macro)] [features] [(*)]`; the features are the last token when it
/// is not a parenthesised note, and a dependency group heading (`[build-dependencies]`) is skipped.
fn parse_tree(output: &str) -> Vec<TreeLine> {
    output
        .lines()
        .filter_map(|line| {
            let mut tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.last() == Some(&"(*)") {
                tokens.pop();
            }
            let name = *tokens.first()?;
            if name.starts_with('[') {
                return None;
            }
            let version = tokens.get(1).copied().unwrap_or_default();
            let features = match tokens.last() {
                Some(last) if tokens.len() > 2 && !last.ends_with(')') => {
                    last.split(',').map(str::to_owned).collect()
                }
                _ => Vec::new(),
            };
            Some(TreeLine {
                name: name.to_owned(),
                version: version.to_owned(),
                features,
            })
        })
        .collect()
}

/// The distinct package names of a tree.
fn tree_names(lines: &[TreeLine]) -> BTreeSet<String> {
    lines.iter().map(|line| line.name.clone()).collect()
}

/// One finding for each forbidden name among `found`.
fn reach_findings(krate: &str, found: &BTreeSet<String>, forbids: &[Forbid]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for forbid in forbids {
        for hit in found
            .iter()
            .filter(|name| forbid_matches(&forbid.name, name))
        {
            let via = if *hit == forbid.name {
                String::new()
            } else {
                format!(" (matches {})", forbid.name)
            };
            findings.push(Finding::new(
                FILE,
                forbid.line,
                format!("{krate} reaches {hit}{via}: {}", forbid.why),
            ));
        }
    }
    findings
}

/// One finding for each forbidden `crate/feature` pair enabled somewhere in the tree.
fn feature_findings(krate: &str, tree: &[TreeLine], forbids: &[Forbid]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for forbid in forbids {
        for line in tree {
            for feature in &line.features {
                let pair = format!("{}/{feature}", line.name);
                if forbid_matches(&forbid.name, &pair) {
                    findings.push(Finding::new(
                        FILE,
                        forbid.line,
                        format!("{krate} enables {pair}: {}", forbid.why),
                    ));
                }
            }
        }
    }
    findings
}

/// One finding for each limit the tree's distinct packages (one per name and version) exceed.
fn budget_findings(krate: &str, tree: &[TreeLine], limits: &[Limit]) -> Vec<Finding> {
    let count = tree
        .iter()
        .map(|line| (&line.name, &line.version))
        .collect::<BTreeSet<_>>()
        .len();
    limits
        .iter()
        .filter(|limit| count > limit.max)
        .map(|limit| {
            Finding::new(
                FILE,
                limit.line,
                format!(
                    "{krate} has {count} packages in its tree, over the budget of {}: {}",
                    limit.max, limit.why
                ),
            )
        })
        .collect()
}

/// Whether `name` is forbidden by `pattern`: equal, or a prefix match when `pattern` ends in `*`.
fn forbid_matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == pattern,
    }
}

/// The optional dependencies (by key) a feature selection enables, and whether each feature is on.
fn enabled_optional(
    features: &BTreeMap<String, Vec<String>>,
    deps: &[Dep],
    selection: &Selection,
) -> BTreeSet<String> {
    let optional: BTreeSet<&str> = deps
        .iter()
        .filter(|dep| dep.optional)
        .map(|dep| dep.key.as_str())
        .collect();
    let mut on_deps = BTreeSet::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<String> = match selection {
        Selection::Default => vec!["default".to_owned()],
        Selection::All => {
            on_deps.extend(optional.iter().map(|key| (*key).to_owned()));
            features.keys().cloned().collect()
        }
        Selection::None => Vec::new(),
        Selection::List(names) => names.clone(),
    };
    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(entries) = features.get(&name) else {
            if optional.contains(name.as_str()) {
                on_deps.insert(name);
            }
            continue;
        };
        for entry in entries {
            if let Some(dep) = entry.strip_prefix("dep:") {
                on_deps.insert(dep.to_owned());
            } else if let Some((dep, _)) = entry.split_once('/') {
                if !dep.ends_with('?') {
                    on_deps.insert(dep.to_owned());
                }
            } else {
                pending.push(entry.clone());
            }
        }
    }
    on_deps
}

/// The packages a member declares as normal (and, with `build`, build) dependencies under `selection`.
fn declared_deps(member: &Member, selection: &Selection, build: bool) -> BTreeSet<String> {
    let on = enabled_optional(&member.features, &member.deps, selection);
    member
        .deps
        .iter()
        .filter(|dep| {
            dep.kind == DependencyKind::Normal || (build && dep.kind == DependencyKind::Build)
        })
        .filter(|dep| !dep.optional || on.contains(&dep.key))
        .map(|dep| dep.name.clone())
        .collect()
}

/// One finding for each `[[edges]]` rule whose declared edges differ from `exactly`.
fn edge_findings(layers: &Layers, members: &[Member], text: &str) -> Vec<Finding> {
    let headers = header_lines(text, "[[edges]]");
    let mut findings = Vec::new();
    for (index, rule) in layers.edges.iter().enumerate() {
        let line = headers.get(index).copied().unwrap_or(1);
        let Some(member) = members.iter().find(|m| m.name == rule.krate) else {
            findings.push(Finding::new(
                FILE,
                line,
                format!("[[edges]] crate `{}` is not a workspace member", rule.krate),
            ));
            continue;
        };
        let mut actual = declared_deps(member, &rule.features, rule.build);
        if rule.scope == Scope::Workspace {
            actual.retain(|name| members.iter().any(|m| &m.name == name));
        }
        let wanted: BTreeSet<String> = rule.exactly.iter().cloned().collect();
        if actual != wanted {
            let list = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>().join(", ");
            findings.push(Finding::new(
                FILE,
                line,
                format!(
                    "{} depends on [{}], expected exactly [{}]: {}",
                    rule.krate,
                    list(&actual),
                    list(&wanted),
                    rule.why
                ),
            ));
        }
    }
    findings
}

/// One finding for each declared dependency a `[[direct]]` rule forbids.
fn direct_findings(layers: &Layers, members: &[Member], text: &str) -> Vec<Finding> {
    let headers = header_lines(text, "[[direct]]");
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
    let mut findings = Vec::new();
    for (index, rule) in layers.direct.iter().enumerate() {
        let line = headers.get(index).copied().unwrap_or(1);
        let targets = select_members(
            "direct",
            rule.krate.as_deref(),
            rule.crates.as_deref(),
            &rule.except,
            &names,
            line,
            &mut findings,
        );
        for target in targets {
            let Some(member) = members.iter().find(|m| m.name == target) else {
                continue;
            };
            for dep in declared_deps(member, &rule.features, rule.build) {
                if rule.allow.iter().any(|ok| forbid_matches(ok, &dep)) {
                    continue;
                }
                if let Some(pattern) = rule.forbid.iter().find(|p| forbid_matches(p, &dep)) {
                    let via = if *pattern == dep {
                        String::new()
                    } else {
                        format!(" (matches {pattern})")
                    };
                    findings.push(Finding::new(
                        FILE,
                        line,
                        format!("{target} depends directly on {dep}{via}: {}", rule.why),
                    ));
                }
            }
        }
    }
    findings
}

/// The package names of a `Cargo.lock`.
fn lock_packages(text: &str) -> Result<BTreeSet<String>> {
    let value: toml::Table = toml::from_str(text).context("Cargo.lock does not parse")?;
    Ok(value
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|package| package.get("name")?.as_str())
        .map(str::to_owned)
        .collect())
}

/// One finding for each package in the lock file that an `[[absent]]` rule names.
fn absent_findings(layers: &Layers, locked: &BTreeSet<String>, text: &str) -> Vec<Finding> {
    let headers = header_lines(text, "[[absent]]");
    let mut findings = Vec::new();
    for (index, rule) in layers.absent.iter().enumerate() {
        let line = headers.get(index).copied().unwrap_or(1);
        for pattern in &rule.names {
            for hit in locked.iter().filter(|name| forbid_matches(pattern, name)) {
                let via = if hit == pattern {
                    String::new()
                } else {
                    format!(" (matches {pattern})")
                };
                findings.push(Finding::new(
                    FILE,
                    line,
                    format!("Cargo.lock has {hit}{via}: {}", rule.why),
                ));
            }
        }
    }
    findings
}

/// One finding for each direct (all features, normal and build) dependency of a listed crate on a
/// crate of a higher tier of the same rule, and one for each listed crate that is not a member.
fn tier_findings(layers: &Layers, members: &[Member], text: &str) -> Vec<Finding> {
    let headers = header_lines(text, "[[tiers]]");
    let mut findings = Vec::new();
    for (index, rule) in layers.tiers.iter().enumerate() {
        let line = headers.get(index).copied().unwrap_or(1);
        let tier_of = |name: &str| {
            rule.tiers
                .iter()
                .position(|tier| tier.iter().any(|c| c == name))
        };
        for (tier, listed) in rule.tiers.iter().enumerate() {
            for name in listed {
                let Some(member) = members.iter().find(|m| &m.name == name) else {
                    findings.push(Finding::new(
                        FILE,
                        line,
                        format!(
                            "[[tiers]] {} lists `{name}`, which is not a workspace member",
                            rule.name
                        ),
                    ));
                    continue;
                };
                for dep in declared_deps(member, &Selection::All, true) {
                    if let Some(higher) = tier_of(&dep).filter(|higher| *higher > tier) {
                        findings.push(Finding::new(
                            FILE,
                            line,
                            format!(
                                "{} {name} (tier {}) depends on {dep} (tier {}): {}",
                                rule.name,
                                tier + 1,
                                higher + 1,
                                rule.why
                            ),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// What a source rule reads beyond the default (non-test `.rs` code outside comments).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ScanOptions {
    /// Globs over paths relative to the root; a match on a directory skips everything under it.
    except: Vec<String>,
    /// Also match comment lines.
    comments: bool,
    /// Also read test files and folders, and code after `#[cfg(test)]`.
    tests: bool,
    /// Read every text file, and report paths that match too.
    all_files: bool,
}

/// Whether `rel` (a path relative to the root, `/`-separated) is `pattern` or under a path that is.
fn excepted(patterns: &[String], rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    patterns.iter().any(|pattern| {
        let wanted: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        !wanted.is_empty()
            && wanted.len() <= parts.len()
            && wanted.iter().zip(&parts).all(|(w, p)| glob_match(w, p))
    })
}

/// The line numbers of `text` that match `regex`. Comment lines are skipped unless `comments`, and
/// reading stops at the first `#[cfg(test)]` line unless `tests`.
fn source_matches(text: &str, regex: &Regex, comments: bool, tests: bool) -> Vec<usize> {
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !tests && trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        if !comments && trimmed.starts_with("//") {
            continue;
        }
        if regex.is_match(line) {
            lines.push(index + 1);
        }
    }
    lines
}

// ---------------------------------------------------------------------------------------------
// The edge: cargo and the file system
// ---------------------------------------------------------------------------------------------

/// The files and directories matching `pattern` (relative to `root`, one glob per segment).
fn expand_paths(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let mut current = vec![PathBuf::new()];
    for segment in pattern.split('/').filter(|s| !s.is_empty()) {
        let mut next = Vec::new();
        for base in &current {
            if segment.contains(['*', '?']) {
                let Ok(entries) = fs::read_dir(root.join(base)) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if glob_match(segment, &name) {
                        next.push(base.join(name));
                    }
                }
            } else if root.join(base).join(segment).exists() {
                next.push(base.join(segment));
            }
        }
        current = next;
    }
    current.sort();
    current
}

/// Whether a directory holds tests, benches or examples, which source rules do not read.
fn is_test_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "tests" | "benches" | "examples"))
}

/// Whether a file is test code by its name (`tests.rs`, `*_tests.rs`, `testkit.rs`).
fn is_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "tests.rs" || name == "testkit.rs" || name.ends_with("_tests.rs")
        })
}

/// Every file under `dir` a source rule reads (sorted) and every path it walks (files and
/// directories, sorted; used for name matches). Test files and folders are left out unless
/// `scan.tests`; `except` paths are left out; only `.rs` files count unless `scan.all_files`.
fn walk_source(root: &Path, dir: &Path, scan: &ScanOptions) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let relative = |path: &Path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let mut files = Vec::new();
    let mut paths = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if excepted(&scan.except, &relative(&path)) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if (!scan.tests && is_test_dir(&path))
                    || (scan.all_files && matches!(name.as_str(), ".git" | "target"))
                {
                    continue;
                }
                paths.push(path.clone());
                stack.push(path);
            } else if (scan.tests || !is_test_file(&path))
                && (scan.all_files || path.extension().is_some_and(|ext| ext == "rs"))
            {
                paths.push(path.clone());
                files.push(path);
            }
        }
    }
    files.sort();
    paths.sort();
    (files, paths)
}

/// Applies one source rule over every file under the directories `path` names.
fn source_findings(
    root: &Path,
    path: &str,
    regex: &Regex,
    why: &str,
    line: usize,
    scan: &ScanOptions,
) -> Vec<Finding> {
    let dirs = expand_paths(root, path);
    let mut findings = Vec::new();
    if dirs.is_empty() {
        findings.push(Finding::new(
            FILE,
            line,
            format!("source path `{path}` matches nothing"),
        ));
    }
    let label = |file: &Path| {
        file.strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string()
    };
    for dir in dirs {
        let full = root.join(&dir);
        if excepted(&scan.except, &dir.to_string_lossy()) {
            continue;
        }
        let (files, paths) = if full.is_file() {
            (vec![full.clone()], vec![full])
        } else {
            walk_source(root, &full, scan)
        };
        if scan.all_files {
            for found in paths {
                let name = label(&found);
                if regex.is_match(&name) {
                    findings.push(Finding::new(name, 1, format!("{why} (the path matches)")));
                }
            }
        }
        for file in files {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            let name = label(&file);
            for hit in source_matches(&text, regex, scan.comments, scan.tests) {
                findings.push(Finding::new(name.clone(), hit, why));
            }
        }
    }
    findings
}

/// Runs a [`TreeJob`] and returns the packages it lists.
fn run_tree(root: &Path, job: &TreeJob) -> Result<Vec<TreeLine>> {
    let out = Command::new("cargo")
        .args(job.args())
        .current_dir(root)
        .output()
        .context("could not start cargo")?;
    if !out.status.success() {
        bail!(
            "cargo {} failed: {}",
            job.args().join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_tree(&String::from_utf8_lossy(&out.stdout)))
}

fn members_of(ws: &Workspace) -> Result<Vec<Member>> {
    let meta = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;
    let mut members = Vec::new();
    for package in &meta.packages {
        let name = package.name.to_string();
        let Some(dir) = ws.dirs.get(&name) else {
            continue;
        };
        members.push(Member {
            manifest: dir.join("Cargo.toml"),
            dir: dir.clone(),
            has_bin: package.targets.iter().any(cargo_metadata::Target::is_bin),
            has_lib: package
                .targets
                .iter()
                .any(|target| target.is_lib() || target.is_proc_macro()),
            deps: package
                .dependencies
                .iter()
                .map(|dep| Dep {
                    name: dep.name.clone(),
                    key: dep.rename.clone().unwrap_or_else(|| dep.name.clone()),
                    kind: dep.kind,
                    optional: dep.optional,
                })
                .collect(),
            features: package.features.clone(),
            name,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(members)
}

/// Layer order between domains, and programs as leaves.
fn order_findings(layers: &Layers, members: &[Member]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let packages: Vec<Package> = members
        .iter()
        .filter_map(|member| {
            Some(Package {
                name: member.name.clone(),
                manifest: member.manifest.clone(),
                domain: domain_of(&member.dir)?,
                program: program_of(layers, member),
            })
        })
        .collect();
    let edges_of = |kinds: &[DependencyKind]| -> Vec<(String, String)> {
        members
            .iter()
            .flat_map(|member| {
                member
                    .deps
                    .iter()
                    .filter(|dep| kinds.contains(&dep.kind))
                    .map(|dep| (member.name.clone(), dep.name.clone()))
            })
            .collect()
    };
    findings.extend(violations(
        &layers.domains,
        &packages,
        &edges_of(&[DependencyKind::Normal]),
    ));
    // A leaf is a program with no library target: a crate with a library and a `bin` is depended on
    // for its library.
    let programs: BTreeSet<String> = members
        .iter()
        .filter(|member| program_of(layers, member) && !member.has_lib)
        .map(|member| member.name.clone())
        .collect();
    let manifests: BTreeMap<String, PathBuf> = members
        .iter()
        .map(|member| (member.name.clone(), member.manifest.clone()))
        .collect();
    findings.extend(leaf_findings(
        &programs,
        &manifests,
        &edges_of(&[DependencyKind::Normal, DependencyKind::Build]),
    ));

    findings
}

/// The rules answered by `cargo tree`: reach, features and budgets.
fn tree_findings(root: &Path, layers: &Layers, names: &[&str], text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let (jobs, rule_findings) = reach_jobs(layers, names, text);
    findings.extend(rule_findings);
    for (job, rules) in &jobs {
        match run_tree(root, job) {
            Ok(tree) => {
                findings.extend(reach_findings(
                    &job.krate,
                    &tree_names(&tree),
                    &rules.forbids,
                ));
                findings.extend(feature_findings(&job.krate, &tree, &rules.features));
                findings.extend(budget_findings(&job.krate, &tree, &rules.limits));
            }
            Err(err) => {
                let line = rules
                    .forbids
                    .iter()
                    .chain(&rules.features)
                    .map(|forbid| forbid.line)
                    .chain(rules.limits.iter().map(|limit| limit.line))
                    .min()
                    .unwrap_or(1);
                findings.push(Finding::new(FILE, line, format!("{err:#}")));
            }
        }
    }

    findings
}

/// The `[[source]]` rules and the kinds' `forbid-src` patterns.
fn all_source_findings(
    root: &Path,
    layers: &Layers,
    members: &[Member],
    text: &str,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let source_headers = header_lines(text, "[[source]]");
    for (index, rule) in layers.source.iter().enumerate() {
        let regex = Regex::new(&rule.pattern)?;
        let line = source_headers.get(index).copied().unwrap_or(1);
        findings.extend(source_findings(
            root,
            &rule.path,
            &regex,
            &rule.why,
            line,
            &rule.scan(),
        ));
    }
    for (crates, rules) in kind_rules(layers) {
        for pattern in rules.forbid_src {
            let regex = Regex::new(pattern)?;
            let why = format!("crates of kind {} may not use this ({pattern})", rules.name);
            for (name, _) in &crates {
                let Some(member) = members.iter().find(|m| &m.name == name) else {
                    continue;
                };
                let src = member.dir.join("src");
                findings.extend(source_findings(
                    root,
                    &src.display().to_string(),
                    &regex,
                    &why,
                    1,
                    &ScanOptions::default(),
                ));
            }
        }
    }

    Ok(findings)
}

/// Checks the rules in `layers.toml` against the workspace.
pub fn run() -> Result<bool> {
    let ws = Workspace::load()?;
    let text = fs::read_to_string(ws.root.join(FILE))
        .with_context(|| format!("could not read {FILE} at the workspace root"))?;
    let layers = Layers::parse(&text)?;
    let members = members_of(&ws)?;
    let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();

    let mut findings = structure_findings(&layers, &members, &text);

    findings.extend(order_findings(&layers, &members));
    findings.extend(edge_findings(&layers, &members, &text));
    findings.extend(direct_findings(&layers, &members, &text));
    findings.extend(tier_findings(&layers, &members, &text));
    if !layers.absent.is_empty() {
        let absent_line = header_lines(&text, "[[absent]]")
            .first()
            .copied()
            .unwrap_or(1);
        match fs::read_to_string(ws.root.join("Cargo.lock"))
            .context("could not read Cargo.lock at the workspace root")
            .and_then(|lock| lock_packages(&lock))
        {
            Ok(locked) => findings.extend(absent_findings(&layers, &locked, &text)),
            Err(err) => findings.push(Finding::new(FILE, absent_line, format!("{err:#}"))),
        }
    }

    findings.extend(tree_findings(&ws.root, &layers, &names, &text));

    findings.extend(all_source_findings(&ws.root, &layers, &members, &text)?);

    findings.sort();
    findings.dedup();
    let (known, findings) = split_known(&layers.known, findings, &text);
    for finding in &known {
        let why = layers
            .known
            .iter()
            .find(|entry| {
                entry
                    .findings
                    .iter()
                    .any(|name| known_text(finding).contains(name.as_str()))
            })
            .map_or("", |entry| entry.why.as_str());
        eprintln!("known: {finding} (known: {why})");
    }
    for finding in &findings {
        eprintln!("{finding}");
    }
    Ok(finish("check layers", findings.is_empty()))
}

/// The text a `[[known]]` entry matches: the finding's path, a space and its message.
fn known_text(finding: &Finding) -> String {
    format!("{} {}", finding.path, finding.message)
}

/// Splits `findings` into the ones a `[[known]]` entry names and the rest; a name that matches no
/// finding is itself one of the rest.
fn split_known(
    known: &[Known],
    findings: Vec<Finding>,
    text: &str,
) -> (Vec<Finding>, Vec<Finding>) {
    let headers = header_lines(text, "[[known]]");
    let names = || known.iter().flat_map(|entry| &entry.findings);
    let (expected, mut rest): (Vec<Finding>, Vec<Finding>) = findings
        .into_iter()
        .partition(|finding| names().any(|name| known_text(finding).contains(name.as_str())));
    for (index, entry) in known.iter().enumerate() {
        for name in &entry.findings {
            if !expected
                .iter()
                .any(|finding| known_text(finding).contains(name.as_str()))
            {
                let line = headers.get(index).copied().unwrap_or(1);
                rest.push(Finding::new(
                    FILE,
                    line,
                    format!("[[known]] `{name}` matches no finding now: delete it"),
                ));
            }
        }
    }
    (expected, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_findings_do_not_fail_and_a_stale_entry_does() {
        let known = [
            Known {
                findings: vec!["layers.toml a reaches b:".to_owned()],
                why: "w".to_owned(),
            },
            Known {
                findings: vec!["src/x.rs".to_owned(), "gone reaches x:".to_owned()],
                why: "w".to_owned(),
            },
        ];
        let findings = vec![
            Finding::new(FILE, 3, "a reaches b: rule"),
            Finding::new(FILE, 4, "c reaches d: rule"),
            Finding::new("src/x.rs", 9, "a source rule"),
        ];
        let text = "[[known]]\nfindings = []\n[[known]]\nfindings = []\n";
        let (expected, rest) = split_known(&known, findings, text);
        assert_eq!(expected.len(), 2);
        let rest: Vec<String> = rest.iter().map(ToString::to_string).collect();
        assert_eq!(
            rest,
            [
                "layers.toml:4: layers: c reaches d: rule",
                "layers.toml:3: layers: [[known]] `gone reaches x:` matches no finding now: delete it",
            ]
        );
    }

    fn domains(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_owned()).collect()
    }

    fn package(name: &str, domain: &str, program: bool) -> Package {
        Package {
            name: name.to_owned(),
            manifest: PathBuf::from(format!("crates/{domain}/{name}/Cargo.toml")),
            domain: domain.to_owned(),
            program,
        }
    }

    fn edge(from: &str, to: &str) -> (String, String) {
        (from.to_owned(), to.to_owned())
    }

    fn dep(name: &str, optional: bool) -> Dep {
        Dep {
            name: name.to_owned(),
            key: name.to_owned(),
            kind: DependencyKind::Normal,
            optional,
        }
    }

    fn member(name: &str, deps: Vec<Dep>, features: &[(&str, &[&str])]) -> Member {
        Member {
            name: name.to_owned(),
            manifest: PathBuf::from(format!("crates/d/{name}/Cargo.toml")),
            dir: PathBuf::from(format!("crates/d/{name}")),
            has_bin: false,
            has_lib: true,
            deps,
            features: features
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
                .collect(),
        }
    }

    const SAMPLE: &str = r#"
domains = ["base", "ui"]

[kind.core]
forbid = ["tokio"]
forbid-src = ['std::fs']

[crate.a]
kind = "core"

[crate.b]
kind = "unclassified"
program = true

[[reach]]
crate = "a"
features = ["x", "y"]
build = false
forbid = ["tokio"]
why = "pure"

[[edges]]
crate = "a"
exactly = ["b"]
why = "exact"

[[source]]
path = "crates/*/src"
pattern = 'async fn'
why = "no async"
"#;

    #[test]
    fn the_file_parses() {
        let layers = Layers::parse(SAMPLE).expect("parses");
        assert_eq!(layers.domains, ["base", "ui"]);
        assert_eq!(layers.krate.len(), 2);
        assert!(layers.krate["b"].program);
        assert_eq!(
            layers.reach[0].features,
            Selection::List(vec!["x".into(), "y".into()])
        );
        assert!(!layers.reach[0].build);
        assert_eq!(layers.edges[0].features, Selection::Default);
        assert_eq!(layers.kind["core"].forbid_src, ["std::fs"]);
    }

    #[test]
    fn unknown_keys_are_an_error() {
        Layers::parse("domains = []\nextra = 1\n").unwrap_err();
        Layers::parse("domains = []\n[crate.a]\nkindd = \"core\"\n").unwrap_err();
        Layers::parse(
            "domains = []\n[[reach]]\ncrate = \"a\"\nforbid = []\nwhy = \"w\"\nbogus = 1\n",
        )
        .unwrap_err();
    }

    #[test]
    fn bad_values_are_an_error() {
        Layers::parse("domains = []\n[crate.a]\nkind = \"nope\"\n").unwrap_err();
        Layers::parse("domains = []\n[[reach]]\ncrate = \"a\"\nfeatures = \"some\"\nforbid = []\nwhy = \"w\"\n").unwrap_err();
        assert!(
            Layers::parse("domains = []\n[[reach]]\nforbid = []\nwhy = \"w\"\n").is_err(),
            "a reach rule needs crate or crates"
        );
        Layers::parse("domains = []\n[[source]]\npath = \"p\"\npattern = \"(\"\nwhy = \"w\"\n")
            .unwrap_err();
    }

    #[test]
    fn globs_match_names() {
        assert!(glob_match("account-*-proto", "account-x-proto"));
        assert!(glob_match("account-*-proto", "account--proto"));
        assert!(!glob_match("account-*-proto", "account-x-protocol"));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn rows_and_members_must_match() {
        let layers = Layers::parse(SAMPLE).expect("parses");
        let members = [
            member("a", vec![], &[]),
            member("c", vec![], &[]),
            Member {
                dir: PathBuf::from("xtask"),
                manifest: PathBuf::from("xtask/Cargo.toml"),
                ..member("xtask", vec![], &[])
            },
        ];
        let lines: Vec<String> = structure_findings(&layers, &members, SAMPLE)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            lines,
            [
                "crates/d/c/Cargo.toml:1: layers: c has no [crate.c] row in layers.toml",
                "xtask/Cargo.toml:1: layers: xtask has no [crate.xtask] row in layers.toml",
                "layers.toml:11: layers: [crate.b] names no workspace member",
            ]
        );
    }

    #[test]
    fn a_domain_is_the_folder_under_crates() {
        assert_eq!(
            domain_of(Path::new("crates/ui/ui-core")).as_deref(),
            Some("ui")
        );
        assert_eq!(domain_of(Path::new("xtask")), None);
    }

    #[test]
    fn a_program_has_a_bin_a_telling_name_or_an_examples_folder() {
        let dir = Path::new("crates/ui/ui-core");
        assert!(is_program("ui-core", dir, true));
        assert!(is_program("account-daemon", dir, false));
        assert!(is_program("computer-use-eval-cloud", dir, false));
        assert!(is_program("x", Path::new("crates/ui/examples/x"), false));
        assert!(!is_program("ui-core", dir, false));
        assert!(!is_program("model-fakery", dir, false));
    }

    #[test]
    fn a_row_can_mark_a_program() {
        let layers = Layers::parse(SAMPLE).expect("parses");
        assert!(program_of(&layers, &member("b", vec![], &[])));
        assert!(!program_of(&layers, &member("a", vec![], &[])));
    }

    #[test]
    fn a_library_may_depend_on_its_own_domain_or_a_lower_one() {
        let d = domains(&["base", "ui", "shell"]);
        let packages = [
            package("a", "ui", false),
            package("b", "ui", false),
            package("c", "base", false),
        ];
        assert!(violations(&d, &packages, &[edge("a", "b"), edge("a", "c")]).is_empty());
    }

    #[test]
    fn a_library_depending_on_a_higher_domain_is_reported() {
        let d = domains(&["base", "ui", "shell"]);
        let packages = [package("a", "ui", false), package("s", "shell", false)];
        let lines: Vec<String> = violations(&d, &packages, &[edge("a", "s")])
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            lines,
            ["crates/ui/a/Cargo.toml:1: layers: layer order: a (ui) -> s (shell)"]
        );
    }

    #[test]
    fn a_program_may_depend_on_any_domain_for_the_order() {
        let d = domains(&["base", "ui", "shell"]);
        let packages = [
            package("a", "ui", true),
            package("s", "shell", false),
            package("l", "base", false),
            package("p", "shell", true),
        ];
        assert!(violations(&d, &packages, &[edge("a", "s"), edge("l", "p")]).is_empty());
    }

    #[test]
    fn a_domain_outside_the_list_is_reported() {
        let d = domains(&["base"]);
        let lines: Vec<String> = violations(&d, &[package("a", "elsewhere", false)], &[])
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            lines,
            [
                "crates/elsewhere/a/Cargo.toml:1: layers: a is in domain `elsewhere`, which `domains` does not name"
            ]
        );
    }

    #[test]
    fn an_edge_to_a_crate_outside_the_workspace_is_ignored_by_the_order() {
        let d = domains(&["ui"]);
        assert!(violations(&d, &[package("a", "ui", false)], &[edge("a", "serde")]).is_empty());
    }

    #[test]
    fn a_dependency_on_a_program_is_reported() {
        let programs: BTreeSet<String> = ["p".to_owned()].into();
        let manifests: BTreeMap<String, PathBuf> = [
            ("a".to_owned(), PathBuf::from("crates/x/a/Cargo.toml")),
            ("p".to_owned(), PathBuf::from("crates/x/p/Cargo.toml")),
        ]
        .into();
        let lines: Vec<String> =
            leaf_findings(&programs, &manifests, &[edge("a", "p"), edge("p", "a")])
                .iter()
                .map(ToString::to_string)
                .collect();
        assert_eq!(
            lines,
            ["crates/x/a/Cargo.toml:1: layers: a depends on the program p; programs are leaves"]
        );
    }

    #[test]
    fn tree_names_are_the_first_token() {
        let out = "a v0.1.0 (/r/a)\nserde v1.0.0\nserde_derive v1.0.0 (proc-macro)\ntokio v1 (*)\n";
        let names = tree_names(&parse_tree(out));
        assert!(names.contains("a") && names.contains("serde_derive") && names.contains("tokio"));
        assert_eq!(names.len(), 4);
    }

    fn tree(out: &str) -> Vec<TreeLine> {
        parse_tree(out)
    }

    #[test]
    fn tree_lines_carry_their_features() {
        let lines = tree(
            "a v0.1.0 (/my dir/a) x,y\nserde v1.0.0 std,derive\nm v1 (proc-macro) f\n\
             t v1 (*)\nu v2 z (*)\nb v1 (/p/b)\n[build-dependencies]\nc v3 \n",
        );
        let features = |name: &str| {
            lines
                .iter()
                .find(|l| l.name == name)
                .map(|l| l.features.clone())
                .expect("listed")
        };
        assert_eq!(features("a"), ["x", "y"]);
        assert_eq!(features("serde"), ["std", "derive"]);
        assert_eq!(features("m"), ["f"]);
        assert!(features("t").is_empty());
        assert_eq!(features("u"), ["z"]);
        assert!(features("b").is_empty());
        assert!(features("c").is_empty());
        assert_eq!(lines.len(), 7);
    }

    #[test]
    fn a_forbidden_feature_is_reported_at_the_rule() {
        let lines = tree("a v1 (/a)\nblitz-dom v1 scrollbars,default\nx v1 other\n");
        let forbids = [
            Forbid {
                name: "blitz-dom/scrollbars".into(),
                why: "no bars".into(),
                line: 4,
            },
            Forbid {
                name: "x/missing".into(),
                why: "w".into(),
                line: 4,
            },
        ];
        let got: Vec<String> = feature_findings("a", &lines, &forbids)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            got,
            ["layers.toml:4: layers: a enables blitz-dom/scrollbars: no bars"]
        );
    }

    #[test]
    fn a_budget_counts_names_and_versions() {
        let lines = tree("a v1 (/a)\nb v1\nb v2\nb v2 (*)\nc v1\n");
        let limits = |max| {
            [Limit {
                max,
                why: "small".into(),
                line: 9,
            }]
        };
        assert!(budget_findings("a", &lines, &limits(4)).is_empty());
        assert_eq!(
            budget_findings("a", &lines, &limits(3))[0].to_string(),
            "layers.toml:9: layers: a has 4 packages in its tree, over the budget of 3: small"
        );
    }

    #[test]
    fn tree_flags_follow_the_selection() {
        let job = |features, build| TreeJob {
            krate: "a".into(),
            features,
            build,
            dev: false,
            prune: Vec::new(),
            host: false,
        };
        assert_eq!(
            job(Selection::Default, false).args().join(" "),
            "tree -p a -e normal --prefix none --target all -f {p} {f}"
        );
        assert!(
            job(Selection::All, true)
                .args()
                .join(" ")
                .ends_with("-e normal,build --prefix none --target all -f {p} {f} --all-features")
        );
        let dev = TreeJob {
            dev: true,
            ..job(Selection::Default, true)
        };
        assert!(dev.args().join(" ").contains("-e normal,build,dev "));
        assert!(
            job(Selection::None, true)
                .args()
                .contains(&"--no-default-features".to_owned())
        );
        assert!(
            job(Selection::List(vec!["a".into(), "b".into()]), true)
                .args()
                .join(" ")
                .ends_with("--no-default-features --features a,b")
        );
    }

    #[test]
    fn reach_rules_group_by_crate_features_and_build() {
        let layers = Layers::parse(
            "domains = []\n\
             [[reach]]\ncrates = \"p-*\"\nforbid = [\"x\"]\nwhy = \"w1\"\n\
             [[reach]]\ncrate = \"p-a\"\nforbid = [\"y\"]\nwhy = \"w2\"\n\
             [[reach]]\ncrate = \"gone\"\nforbid = [\"y\"]\nwhy = \"w3\"\n",
        )
        .expect("parses");
        let text = "domains = []\n[[reach]]\n\n\n\n\n[[reach]]\n\n\n\n\n[[reach]]\n";
        let (jobs, findings) = reach_jobs(&layers, &["p-a", "p-b", "q"], text);
        assert_eq!(jobs.len(), 2);
        let job = TreeJob {
            krate: "p-a".into(),
            features: Selection::Default,
            build: true,
            dev: false,
            prune: Vec::new(),
            host: false,
        };
        assert_eq!(jobs[&job].forbids.len(), 2);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 12);
    }

    #[test]
    fn a_forbidden_name_found_is_reported_at_the_rule() {
        let found: BTreeSet<String> = ["serde".to_owned(), "tokio".to_owned()].into();
        let forbids = [
            Forbid {
                name: "tokio".into(),
                why: "pure".into(),
                line: 7,
            },
            Forbid {
                name: "zbus".into(),
                why: "pure".into(),
                line: 7,
            },
        ];
        let lines: Vec<String> = reach_findings("a", &found, &forbids)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(lines, ["layers.toml:7: layers: a reaches tokio: pure"]);
    }

    #[test]
    fn kind_rules_apply_to_classified_crates_only() {
        let layers = Layers::parse(SAMPLE).expect("parses");
        let (jobs, _) = reach_jobs(&layers, &["a", "b"], SAMPLE);
        let kind_job = TreeJob {
            krate: "a".into(),
            features: Selection::Default,
            build: true,
            dev: false,
            prune: Vec::new(),
            host: false,
        };
        assert!(jobs.contains_key(&kind_job));
        assert!(!jobs.keys().any(|job| job.krate == "b"));
    }

    #[test]
    fn optional_dependencies_follow_the_features() {
        let features: &[(&str, &[&str])] = &[
            ("default", &["fast"]),
            ("fast", &["dep:o1"]),
            ("slow", &["o2/extra"]),
            ("weak", &["o3?/extra"]),
            ("both", &["slow", "o4"]),
        ];
        let deps = vec![
            dep("m", false),
            dep("o1", true),
            dep("o2", true),
            dep("o3", true),
            dep("o4", true),
        ];
        let m = member("a", deps, features);
        let set = |items: &[&str]| -> BTreeSet<String> {
            items.iter().map(|s| (*s).to_owned()).collect()
        };
        assert_eq!(
            declared_deps(&m, &Selection::Default, false),
            set(&["m", "o1"])
        );
        assert_eq!(declared_deps(&m, &Selection::None, false), set(&["m"]));
        assert_eq!(
            declared_deps(&m, &Selection::List(vec!["both".into()]), false),
            set(&["m", "o2", "o4"])
        );
        assert_eq!(
            declared_deps(&m, &Selection::List(vec!["weak".into()]), false),
            set(&["m"])
        );
        assert_eq!(
            declared_deps(&m, &Selection::All, false),
            set(&["m", "o1", "o2", "o3", "o4"])
        );
    }

    #[test]
    fn dev_dependencies_are_not_edges_and_build_ones_need_the_flag() {
        let mut dev = dep("d", false);
        dev.kind = DependencyKind::Development;
        let mut build = dep("bld", false);
        build.kind = DependencyKind::Build;
        let m = member(
            "a",
            vec![dev, build, dep("serde", false), dep("w", false)],
            &[],
        );
        let got = declared_deps(&m, &Selection::Default, false);
        assert_eq!(got.into_iter().collect::<Vec<_>>(), ["serde", "w"]);
        let got = declared_deps(&m, &Selection::Default, true);
        assert_eq!(got.into_iter().collect::<Vec<_>>(), ["bld", "serde", "w"]);
    }

    #[test]
    fn a_forbid_name_may_be_a_prefix() {
        assert!(forbid_matches("tokio*", "tokio"));
        assert!(forbid_matches("tokio*", "tokio-util"));
        assert!(!forbid_matches("tokio", "tokio-util"));
        assert!(!forbid_matches("tokio*", "mio"));
        let found: BTreeSet<String> = ["tokio-util".to_owned()].into();
        let forbids = [Forbid {
            name: "tokio*".into(),
            why: "w".into(),
            line: 3,
        }];
        assert_eq!(
            reach_findings("a", &found, &forbids)[0].to_string(),
            "layers.toml:3: layers: a reaches tokio-util (matches tokio*): w"
        );
    }

    #[test]
    fn a_glob_rule_skips_its_exceptions() {
        let layers = Layers::parse(
            "domains = []\n[[reach]]\ncrates = \"p-*\"\nexcept = [\"p-b\"]\nforbid = [\"x\"]\nwhy = \"w\"\n",
        )
        .expect("parses");
        let (jobs, _) = reach_jobs(&layers, &["p-a", "p-b"], "");
        assert_eq!(
            jobs.keys().map(|j| j.krate.as_str()).collect::<Vec<_>>(),
            ["p-a"]
        );
        Layers::parse(
            "domains = []\n[[reach]]\ncrate = \"a\"\nexcept = [\"b\"]\nforbid = []\nwhy = \"w\"\n",
        )
        .unwrap_err();
    }

    #[test]
    fn source_patterns_may_carry_flags() {
        let regex = Regex::new(r"(?i)\bfoo\b").expect("regex");
        assert_eq!(source_matches("let FOO = 1;\n", &regex, false, false), [1]);
    }

    #[test]
    fn edges_are_compared_exactly() {
        let layers = Layers::parse(SAMPLE).expect("parses");
        let a = member("a", vec![dep("b", false)], &[]);
        let b = member("b", vec![], &[]);
        let text = "[[edges]]\n";
        assert!(edge_findings(&layers, &[a, b.clone()], text).is_empty());
        let a = member("a", vec![], &[]);
        let lines: Vec<String> = edge_findings(&layers, &[a, b], text)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            lines,
            ["layers.toml:1: layers: a depends on [], expected exactly [b]: exact"]
        );
    }

    #[test]
    fn source_skips_comments_and_stops_at_the_test_module() {
        let regex = Regex::new(r"\basync\s+fn\b").expect("regex");
        let text = "async fn a() {}\n// async fn b\n    /// async fn c\nfn d() {}\npub async  fn e() {}\n#[cfg(test)]\nmod t { async fn f() {} }\n";
        assert_eq!(source_matches(text, &regex, false, false), [1, 5]);
    }

    #[test]
    fn source_can_read_comments_and_tests() {
        let regex = Regex::new(r"\basync\s+fn\b").expect("regex");
        let text = "async fn a() {}\n// async fn b\n#[cfg(test)]\nmod t { async fn f() {} }\n";
        assert_eq!(source_matches(text, &regex, true, false), [1, 2]);
        assert_eq!(source_matches(text, &regex, false, true), [1, 4]);
        assert_eq!(source_matches(text, &regex, true, true), [1, 2, 4]);
    }

    #[test]
    fn except_globs_skip_a_path_and_everything_under_it() {
        let skip = |p: &[&str], rel: &str| {
            excepted(&p.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(), rel)
        };
        assert!(skip(&["crates/p/time.rs"], "crates/p/time.rs"));
        assert!(skip(&["crates/*/*/src/bin"], "crates/a/b/src/bin/x.rs"));
        assert!(skip(&["crates/*/*/src/main.rs"], "crates/a/b/src/main.rs"));
        assert!(!skip(&["crates/*/*/src/bin"], "crates/a/b/src/binary.rs"));
        assert!(!skip(&["crates/a/b"], "crates/a"));
        assert!(!skip(&[], "crates/a"));
    }

    #[test]
    fn source_options_parse() {
        let layers = Layers::parse(
            "domains = []\n[[source]]\npath = \"p\"\npattern = \"x\"\nexcept = [\"a/*\"]\ncomments = true\ntests = true\nfiles = \"all\"\nwhy = \"w\"\n",
        )
        .expect("parses");
        assert_eq!(
            layers.source[0].scan(),
            ScanOptions {
                except: vec!["a/*".into()],
                comments: true,
                tests: true,
                all_files: true,
            }
        );
        Layers::parse(
            "domains = []\n[[source]]\npath = \"p\"\npattern = \"x\"\nfiles = \"some\"\nwhy = \"w\"\n",
        )
        .unwrap_err();
    }

    #[test]
    fn new_reach_fields_parse() {
        let layers = Layers::parse(
            "domains = []\n[[reach]]\ncrate = \"a\"\ndev = true\nforbid-features = [\"x/f\"]\nwhy = \"w\"\n",
        )
        .expect("parses");
        assert!(layers.reach[0].dev);
        let (jobs, _) = reach_jobs(&layers, &["a"], "[[reach]]\n");
        let (job, rules) = jobs.iter().next().expect("one job");
        assert!(job.dev);
        assert_eq!(rules.features[0].name, "x/f");
        assert!(rules.forbids.is_empty());
    }

    #[test]
    fn direct_rules_read_declared_dependencies() {
        let layers = Layers::parse(
            "domains = []\n[[direct]]\ncrates = \"f-*\"\nexcept = [\"f-skip\"]\nforbid = [\"tokio*\", \"ui-*\"]\nallow = [\"ui-helpers\"]\nwhy = \"w\"\n\
             [[direct]]\ncrate = \"gone\"\nforbid = []\nwhy = \"w\"\n",
        )
        .expect("parses");
        let deps = || {
            vec![
                dep("tokio-util", false),
                dep("ui-helpers", false),
                dep("ui-kit", false),
                dep("serde", false),
                dep("tokio", true),
            ]
        };
        let members = [
            member("f-a", deps(), &[]),
            member("f-skip", deps(), &[]),
            member("g", deps(), &[]),
        ];
        let got: Vec<String> = direct_findings(
            &layers,
            &members,
            "[[direct]]\n\n\n\n\n\n\n\n\n[[direct]]\n",
        )
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            got,
            [
                "layers.toml:1: layers: f-a depends directly on tokio-util (matches tokio*): w",
                "layers.toml:1: layers: f-a depends directly on ui-kit (matches ui-*): w",
                "layers.toml:10: layers: [[direct]] crate `gone` is not a workspace member",
            ]
        );
        Layers::parse("domains = []\n[[direct]]\nforbid = []\nwhy = \"w\"\n").unwrap_err();
    }

    #[test]
    fn absent_rules_read_the_lock_file() {
        let lock = "version = 4\n[[package]]\nname = \"smithay\"\nversion = \"1\"\n[[package]]\nname = \"smithay-client-toolkit\"\n[[package]]\nname = \"serde\"\n";
        let locked = lock_packages(lock).expect("parses");
        assert_eq!(locked.len(), 3);
        let layers = Layers::parse(
            "domains = []\n[[absent]]\nnames = [\"smithay*\", \"niri\"]\nwhy = \"no compositor\"\n",
        )
        .expect("parses");
        let got: Vec<String> = absent_findings(&layers, &locked, "[[absent]]\n")
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            got,
            [
                "layers.toml:1: layers: Cargo.lock has smithay (matches smithay*): no compositor",
                "layers.toml:1: layers: Cargo.lock has smithay-client-toolkit (matches smithay*): no compositor",
            ]
        );
    }

    #[test]
    fn tiers_only_depend_downward() {
        let layers = Layers::parse(
            "domains = []\n[[tiers]]\nname = \"acts\"\ntiers = [[\"core\"], [\"client\"], [\"daemon\", \"gone\"]]\nwhy = \"pure first\"\n",
        )
        .expect("parses");
        let members = [
            member("core", vec![dep("client", false), dep("serde", false)], &[]),
            member(
                "client",
                vec![dep("core", false), dep("daemon", true)],
                &[("f", &["dep:daemon"])],
            ),
            member("daemon", vec![dep("client", false)], &[]),
        ];
        let got: Vec<String> = tier_findings(&layers, &members, "[[tiers]]\n")
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            got,
            [
                "layers.toml:1: layers: acts core (tier 1) depends on client (tier 2): pure first",
                "layers.toml:1: layers: acts client (tier 2) depends on daemon (tier 3): pure first",
                "layers.toml:1: layers: [[tiers]] acts lists `gone`, which is not a workspace member",
            ]
        );
        Layers::parse(
            "domains = []\n[[tiers]]\nname = \"t\"\ntiers = [[\"a\"], [\"a\"]]\nwhy = \"w\"\n",
        )
        .unwrap_err();
    }
}
