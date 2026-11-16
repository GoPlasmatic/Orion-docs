//! The release stamps that name the Orion version by hand.
//!
//! The book documents one Orion release: the one every `orion-*` dependency
//! in `drift/Cargo.toml` is pinned to. Executable tutorials state the version
//! they were run with ("Tested with Orion 2.0.0."), and `lint.sh` check 18
//! compares them with that pin without building anything. This holds the
//! chain to what cargo actually resolved: the pinned tag names the resolved
//! version, and every stamp anywhere in the book names it too.

use crate::support::{docs_path, orion_version};

/// The `tag = "…"` on every Orion git dependency in `drift/Cargo.toml`.
fn pinned_tags() -> Vec<String> {
    let manifest = std::fs::read_to_string(docs_path("drift/Cargo.toml")).unwrap();
    let marker = "git = \"https://github.com/GoPlasmatic/Orion\", tag = \"";
    manifest
        .match_indices(marker)
        .map(|(at, _)| {
            let rest = &manifest[at + marker.len()..];
            rest[..rest.find('"').expect("a closed tag string")].to_string()
        })
        .collect()
}

#[test]
fn the_pinned_tag_is_the_resolved_release() {
    let tags = pinned_tags();
    assert!(
        tags.len() >= 5,
        "found {} Orion dependencies in drift/Cargo.toml — the parser lost them",
        tags.len()
    );
    let expected = format!("v{}", orion_version());
    let stray: Vec<&String> = tags.iter().filter(|t| **t != expected).collect();
    assert!(
        stray.is_empty(),
        "every orion-* dependency must be pinned to {expected} (the version cargo \
         resolved); found {stray:?}. One tag across them is what lets lint.sh and \
         the tutorials name a single release."
    );
}

#[test]
fn every_tested_with_stamp_names_the_pinned_release() {
    let version = orion_version();
    let mut stamps = 0;
    let mut stale = Vec::new();
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
            for (n, line) in text.lines().enumerate() {
                let mut rest = line;
                while let Some(at) = rest.find("Tested with Orion ") {
                    rest = &rest[at + "Tested with Orion ".len()..];
                    let stated = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches('.');
                    stamps += 1;
                    if stated != version {
                        stale.push(format!(
                            "{}:{}: tested with {stated}",
                            path.strip_prefix(docs_path("")).unwrap_or(&path).display(),
                            n + 1
                        ));
                    }
                }
            }
        }
    }
    assert!(
        stamps >= 10,
        "found only {stamps} `Tested with Orion` stamps — the scan lost them"
    );
    assert!(
        stale.is_empty(),
        "these pages name a release other than the pinned Orion {version}:\n  {}",
        stale.join("\n  ")
    );
}
