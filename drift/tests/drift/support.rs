//! Where things are: this repository's files, and the Orion release the
//! guards are built against.
//!
//! Orion's crates arrive as a git dependency, so their sources sit in cargo's
//! git checkout (or, under a local `[patch]`, in an Orion working tree) —
//! never at a path relative to this repository. `cargo metadata` is the one
//! thing that knows which, so every guard reading an Orion source file asks it
//! through [`crate_dir`] rather than guessing.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The root of this repository: the book's `book.toml`, `src/`, `openapi.json`.
pub fn docs_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("drift/ sits in the documentation repository")
        .to_path_buf()
}

/// A path under this repository's root.
pub fn docs_path(relative: &str) -> PathBuf {
    docs_root().join(relative)
}

/// One Orion package as `cargo metadata` resolved it.
struct Package {
    name: String,
    version: String,
    manifest_path: PathBuf,
}

/// The resolved dependency graph's `orion-*` packages, read once per run.
fn orion_packages() -> &'static [Package] {
    static PACKAGES: OnceLock<Vec<Package>> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let output = std::process::Command::new(cargo)
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&manifest)
            .output()
            .expect("run `cargo metadata`");
        assert!(
            output.status.success(),
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("`cargo metadata` prints JSON");
        let packages: Vec<Package> = metadata["packages"]
            .as_array()
            .expect("metadata lists packages")
            .iter()
            .filter_map(|p| {
                let name = p["name"].as_str()?;
                name.starts_with("orion-").then(|| Package {
                    name: name.to_string(),
                    version: p["version"].as_str().unwrap_or_default().to_string(),
                    manifest_path: PathBuf::from(p["manifest_path"].as_str().unwrap_or_default()),
                })
            })
            .filter(|p| p.name != "orion-docs-drift")
            .collect();
        assert!(
            packages.iter().any(|p| p.name == "orion-common"),
            "`cargo metadata` resolved no orion-common — is the Orion dependency declared?"
        );
        packages
    })
}

/// The directory of an Orion crate in the release the guards are built against.
///
/// A crate in the dependency graph is found by its resolved manifest. One that
/// is not — `orion-server`, whose sources some guards scan but nothing here
/// links — is found in the same Orion checkout, and its manifest must name it.
pub fn crate_dir(name: &str) -> PathBuf {
    if let Some(package) = orion_packages().iter().find(|p| p.name == name) {
        return package
            .manifest_path
            .parent()
            .expect("a manifest has a directory")
            .to_path_buf();
    }
    let dir = orion_root().join("crates").join(name);
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_else(|e| {
        panic!(
            "{name} is not in the Orion checkout at {}: {e}",
            dir.display()
        )
    });
    assert!(
        manifest.contains(&format!("name = \"{name}\"")),
        "{} is not the {name} package",
        dir.display()
    );
    dir
}

/// `src/` of an Orion crate.
pub fn crate_src(name: &str) -> PathBuf {
    crate_dir(name).join("src")
}

/// The root of the Orion checkout the dependencies were resolved from — the
/// workspace two levels above `crates/orion-common`.
pub fn orion_root() -> PathBuf {
    let common = orion_packages()
        .iter()
        .find(|p| p.name == "orion-common")
        .expect("orion-common is resolved");
    let root = common
        .manifest_path
        .ancestors()
        .nth(3)
        .expect("crates/orion-common/Cargo.toml sits two levels below the workspace root")
        .to_path_buf();
    let workspace = std::fs::read_to_string(root.join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("no Orion workspace manifest at {}: {e}", root.display()));
    assert!(
        workspace.contains("[workspace]"),
        "{} is not the Orion workspace root",
        root.display()
    );
    root
}

/// The Orion version cargo resolved — every orion-* crate shares one.
pub fn orion_version() -> String {
    let versions: std::collections::BTreeSet<&str> = orion_packages()
        .iter()
        .map(|p| p.version.as_str())
        .collect();
    assert_eq!(
        versions.len(),
        1,
        "the orion-* crates resolved to more than one version: {versions:?}"
    );
    versions.into_iter().next().unwrap().to_string()
}
