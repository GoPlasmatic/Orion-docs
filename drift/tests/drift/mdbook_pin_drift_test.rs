//! Drift guard for the mdBook version pin.
//!
//! The docs site is deployed by Cloudflare Workers Builds, which clones the
//! repo and runs `bash build.sh`. That script pins `MDBOOK_VERSION` and
//! fetches the matching release binary — but only when a suitable mdbook is
//! not already on PATH, which is exactly the case on Cloudflare and exactly
//! *not* the case in CI, where the book job in `.github/workflows/docs.yml`
//! installs mdbook itself.
//!
//! So the two versions are used by two different machines and neither one
//! ever sees the other. Let them drift and the failure is silent in the worst
//! way: the PR check goes green having built the book with one mdbook, and
//! Cloudflare then publishes a site built with another. No error, no diff to
//! review — just a live site nothing verified.
//!
//! This test is the thing that makes the workflow's "the PR check must
//! exercise the exact mdbook that deploys" comment true, rather than merely
//! intended.

use crate::support::docs_path;

/// `MDBOOK_VERSION=0.5.4` in build.sh — the version Cloudflare deploys.
fn build_script_pin(script: &str) -> Option<&str> {
    script.lines().find_map(|line| {
        line.trim()
            .strip_prefix("MDBOOK_VERSION=")
            .map(|v| v.trim().trim_matches('"'))
    })
}

/// `tool: mdbook@0.5.4` in the workflow's book job — the version PRs are
/// checked against.
fn workflow_pin(workflow: &str) -> Option<&str> {
    workflow
        .lines()
        .find_map(|line| line.trim().strip_prefix("tool: mdbook@"))
        .map(str::trim)
}

#[test]
fn the_deployed_mdbook_is_the_one_ci_checks() {
    let script = std::fs::read_to_string(docs_path("build.sh"))
        .expect("build.sh is the deploy entry point and must exist");
    let workflow = std::fs::read_to_string(docs_path(".github/workflows/docs.yml"))
        .expect(".github/workflows/docs.yml must exist");

    let deployed = build_script_pin(&script).expect(
        "build.sh no longer declares `MDBOOK_VERSION=`. That line is the \
         only place the deployed mdBook version is written down — Cloudflare \
         Workers Builds reads it by running the script. Restore it, or this \
         guard is checking nothing.",
    );
    let checked = workflow_pin(&workflow).expect(
        "the docs.yml book job no longer installs `tool: mdbook@<version>`. \
         Without it the PR check builds the book with whatever mdbook the \
         runner happens to have.",
    );

    assert_eq!(
        deployed, checked,
        "mdBook pin drift: build.sh deploys the docs with {deployed}, but \
         docs.yml checks pull requests with {checked}. Whichever is right, both \
         must say it — otherwise the site Cloudflare publishes was built by a \
         version no PR ever ran."
    );
}
