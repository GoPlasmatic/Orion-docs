//! The example files the book includes are the pinned release's.
//!
//! A few pages `{{#include}}` whole definition files from Orion's
//! `examples/packages/` — the shipped, CI-executed packages. mdBook can only
//! include files inside this repository, so those files are vendored under
//! `examples/` at the same relative path, and this holds each one byte-equal
//! to the Orion checkout cargo resolved. A stale copy would show readers a
//! definition the release does not ship.

use std::path::{Path, PathBuf};

use crate::support::{docs_path, orion_root};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read examples dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn vendored_examples_match_the_pinned_release() {
    let vendored_root = docs_path("examples");
    let mut vendored = Vec::new();
    files(&vendored_root, &mut vendored);
    vendored.sort();
    assert!(!vendored.is_empty(), "examples/ holds no files");

    let orion = orion_root();
    let mut stale = Vec::new();
    for path in &vendored {
        let relative = path.strip_prefix(&vendored_root).unwrap();
        let upstream = orion.join("examples").join(relative);
        match std::fs::read(&upstream) {
            Ok(bytes) if bytes == std::fs::read(path).unwrap() => {}
            Ok(_) => stale.push(format!("examples/{} differs", relative.display())),
            Err(_) => stale.push(format!(
                "examples/{} is not in the pinned release",
                relative.display()
            )),
        }
    }
    assert!(
        stale.is_empty(),
        "vendored examples have drifted from Orion's examples/ (copy them again):\n  {}",
        stale.join("\n  ")
    );
}

/// Every vendored file is one a page includes, and every include of an
/// example resolves to a vendored file — so the directory holds exactly what
/// the book uses.
#[test]
fn every_vendored_example_is_included_by_a_page() {
    let mut included = std::collections::BTreeSet::new();
    let mut stack = vec![docs_path("src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (at, _) in text.match_indices("{{#include ") {
                let rest = &text[at + "{{#include ".len()..];
                let target = rest[..rest.find("}}").unwrap()]
                    .split(':')
                    .next()
                    .unwrap()
                    .trim();
                if target.contains("examples/") {
                    let resolved = path
                        .parent()
                        .unwrap()
                        .join(target)
                        .canonicalize()
                        .unwrap_or_else(|_| {
                            panic!("{}: include target missing: {target}", path.display())
                        });
                    included.insert(resolved);
                }
            }
        }
    }
    let mut vendored = Vec::new();
    files(&docs_path("examples"), &mut vendored);
    let unused: Vec<String> = vendored
        .iter()
        .map(|p| p.canonicalize().unwrap())
        .filter(|p| !included.contains(p))
        .map(|p| p.display().to_string())
        .collect();
    assert!(
        unused.is_empty(),
        "examples/ holds files no page includes: {unused:?}"
    );
}
