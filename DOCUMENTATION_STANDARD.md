# Orion Documentation Standard

This document defines how Orion documentation is structured, written, and maintained. Every rule here exists to help developers understand Orion quickly, build working services, and operate them reliably in production.

For Orion-specific terminology and formatting rules, see [`STYLE_GUIDE.md`](./STYLE_GUIDE.md). For the site architecture, page map, and build/test workflow, see [`CONTRIBUTING-DOCS.md`](./CONTRIBUTING-DOCS.md).

---

## 1. Core Principles

### 1.1 Write in plain, natural developer English
- Use clear, direct sentences. Write the way an experienced engineer explains a system to a teammate.
- Do not write in riddles, aphorisms, or clipped telegraphic fragments. If a concept takes a 30-word sentence or a short paragraph to explain clearly, write it naturally.
- Use **Standard American English** (`behavior`, `color`, `license`, `catalog`, `normalize`, `canceled`, `artifact`). Keep exact wire spellings inside code spans (for example, `[extensions.licence]` in `orion-ext` configuration).

### 1.2 Eliminate AI slop and marketing fluff
<!-- vale off -->
- **No marketing buzzwords**: Never use words like *blazing, seamless, effortlessly, robust, enterprise-grade, best-in-class, cutting-edge, supercharge, unlock,* or *leverage*.
- **No throat-clearing filler**: Cut openings like *"In this comprehensive guide, we will explore..."*, *"It is worth noting that..."*, *"At its core..."*, or *"Let's dive in."* Start directly with the technical fact or action.
- **No condescending ease words**: Avoid telling the reader something is *simple, simply, easy, easily, obviously, trivial,* or *merely* when describing a procedure. State the steps and let the reader judge.
<!-- vale on -->
- **Be honest about boundaries**: State what Orion does and what it does not do with equal clarity. When a feature requires Orion Standard or Enterprise (`orion-ext`), or when another tool is a better fit, say so up front.

### 1.3 Focus on how developers actually use Orion
- **Files and CLI first**: Developers build Orion services by writing `workflow.json`, `channel.json`, and `connector.json` files on disk, testing them offline with `orion-cli`, compiling them with `orion-cli compile`, and deploying them with `orion-cli deployments apply -d`.
- **Wire API second**: The `/api/v2/admin/*` HTTP endpoints are the underlying contract that `orion-cli` and the Orion Console call. Show `orion-cli` as the primary path in tutorials and guides, and provide `curl` examples as the secondary tab or in the REST API reference.

### 1.4 Organize for strong flow and scannability
- **Cohesive pages over micro-fragments**: Group closely related concepts, configuration keys, functions, or commands onto cohesive, scannable pages so developers can read top-to-bottom or search with `Cmd+F` without clicking through dozens of tiny files.
- **Progressive disclosure**: Move from the common 80% path to advanced options, edge cases, and failure modes.
- **Every page is a landing page**: Assume the reader arrived from search or an AI assistant link. Open every page with a clear 1–3 sentence overview stating what the page covers and what the reader will accomplish.

---

## 2. Content Types

Every page declares its content type in an HTML comment on line 2 (`<!-- type: <type> -->`). Each type serves a distinct developer need:

| Type | Reader's Goal | What It Contains |
| :--- | :--- | :--- |
| **`quickstart`** | "Show me Orion working in 5 minutes." | Prerequisites, numbered copy-pasteable steps using `orion-cli` and definition files, expected output, verification, and next steps. |
| **`tutorial`** | "Teach me how to build a real service end to end." | What you will build, prerequisites, ordered steps that produce a working service, offline testing, live verification, and cleanup. |
| **`concept`** | "Help me understand how this part of Orion works." | Clear definition, the problem it solves, architecture/mental model (with diagrams and minimal JSON examples), key behaviors, and design boundaries. |
| **`guide`** | "Show me how to complete a specific task." | Task overview, prerequisites, practical examples and step-by-step instructions, common patterns, verification, and troubleshooting tips. |
| **`reference`** | "Give me the exact schema, flags, fields, or error codes." | Exact syntax/synopsis, complete parameter/field tables (with types, requiredness, defaults, and descriptions), return values, error codes, and concrete examples. |
| **`troubleshooting`** | "Something broke—how do I diagnose and fix it?" | Symptom-based headings (exact error messages or observable behaviors), root cause explanation, step-by-step fix, and verification. |
| **`hub`** | "Orient me at the root of the documentation." | Concise product overview, quick entry points by developer goal, and a map of the documentation sections. |
| **`migration`** / **`release-note`** | "What changed between versions and how do I upgrade?" | Summary of changes, breaking changes table, before/after examples, and step-by-step upgrade instructions. |
| **`glossary`** | "What does this Orion term mean?" | Alphabetical definitions of Orion's core nouns and concepts. |

---

## 3. Information Architecture

### 3.1 The Seven-Part Spine
The documentation is organized into seven top-level parts that mirror the developer lifecycle:

1. **Get started**: See Orion work in minutes, install the tools, build your first services, and evaluate where Orion fits.
2. **Core concepts**: Understand the mental model—request lifecycle, workflows, channels, connectors, deployments, and editions.
3. **Build services**: Practical guides for authoring workflows, configuring channels and guards, connecting databases and APIs, scheduling cron jobs, testing offline, setting up CI/CD, writing WebAssembly plugins, and authoring with AI.
4. **Standard & Enterprise**: Dedicated guides and reference for `orion-ext` capabilities—Kafka, GraphQL, and SSE channels, the seven enterprise connector providers, canary traffic splitting, multi-node Redis clustering, and custom provider packages.
5. **Operate in production**: Production readiness checklist, Docker/Kubernetes/HA deployment topologies, security hardening, monitoring and metrics, traces and audit logs, resilience tuning, backups, upgrades, and troubleshooting.
6. **Reference**: Complete technical specifications—workflow and channel schemas, task functions, JSONLogic expressions, connector schemas and portable data dialect, CLI commands, REST API endpoints, server configuration (`orion.toml` and environment variables), error codes, and metrics.
7. **Releases**: Versioning and support policy, the Orion 2.0 upgrade guide, and historical 1.x release notes.

### 3.2 Navigation & URLs
- **Maximum sidebar depth**: Keep navigation to `Part → Page` (or `Part → Group → Page` where needed). Never nest deeper than 3 levels.
- **Title consistency**: A page's `# H1` title, browser `<title>`, and `llms.txt` entry should match its sidebar label in `SUMMARY.md` (or use a natural prefix like `Tutorial: `).
- **Never break inbound URLs**: Whenever pages are consolidated, moved, or renamed, add a redirect entry under `[output.html.redirect]` in `book.toml`.

---

## 4. Voice, Tone, and Formatting

### 4.1 Voice and Tense
- Address the reader as **you**. Never refer to the reader as "the user".
- Use **present tense** for product behavior: *"Orion validates the payload before running the workflow"* (not *"Orion will validate"*).
- Use **imperative verbs** for instructions: *"Run `orion-cli compile`"* (not *"You should run"*).
- Use **active voice** and name the specific component acting: *"The channel rejects the request with HTTP `401`"* or *"The CLI uploads each definition"*.
- Contractions (*don't, can't, it's, you'll*) are welcome in guides, tutorials, and concepts where they make prose sound natural.

### 4.2 Headings, Lists, and Tables
- Use **sentence case** for all headings (`# Configure channels and guards`, not `# Configure Channels And Guards`). Capitalize proper nouns and product names (`Orion`, `PostgreSQL`, `Kafka`, `JSONLogic`, `WebAssembly`).
- Do not put periods, colons, or exclamation marks at the end of headings (question marks are fine for FAQ/troubleshooting headings).
- Do not skip heading levels (`H1 → H2 → H3 → H4`). Do not nest deeper than `H4`.
- Use **tables** whenever comparing options or listing structured attributes (fields, flags, error codes, metrics).
- Standard field/parameter table columns:
  `| Field | Type | Required | Default | Description |`
  Use lowercase `yes` / `no` in the `Required` column and `—` when no default applies.

### 4.3 Callouts and Edition Badges
- Use GitHub-flavored Markdown alerts sparingly for information that genuinely needs visual emphasis:
  - `> [!NOTE]` — Helpful context or an important distinction.
  - `> [!TIP]` — A practical shortcut or best practice.
  - `> [!IMPORTANT]` — Key requirement needed for a task to succeed.
  - `> [!WARNING]` — Security risk, data loss hazard, breaking behavior, or a non-obvious runtime trap.
- Do not place two callouts back-to-back.
- **Edition marking**: Community is the default across the documentation. Mark features that require Orion Standard or Enterprise (`orion-ext`) with the `Standard+` badge beside the section heading or table row, and state the requirement clearly in prose:
  ```html
  <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>
  ```
  Never place HTML badges inside the `# H1` title.

---

## 5. Code Samples and Diagrams

### 5.1 Code Samples
- **Runnable and realistic**: Every JSON definition, TOML snippet, and CLI command must be valid and match the current Orion release.
- **No shell prompts**: Omit `$` prompts from shell commands so readers can copy and paste directly. Show command output in a separate block below the command.
- **Tabs for equivalent surfaces**: When showing both the CLI and raw HTTP API, use `<div class="tabs">` in **CLI → curl** order:
  ```html
  <div class="tabs">
  <section data-tab="CLI">

  ```bash
  orion-cli send orders -d '{"order_id": "ORD-1001", "total": 25000}'
  ```

  </section>
  <section data-tab="curl">

  ```bash
  curl -sS -X POST http://localhost:8080/api/v2/data/orders \
    -H "Content-Type: application/json" \
    -d '{"data": {"order_id": "ORD-1001", "total": 25000}}'
  ```

  </section>
  </div>
  ```
- **Safe placeholders**: Never include real credentials. Use `env://VARIABLE_NAME` in connector definitions, `$ORION_ADMIN_KEY` in shell commands, and RFC-reserved domains (`example.com`) and IP ranges (`203.0.113.0/24`).

### 5.2 Diagrams
- Use `orion-diagram` or `orion-mindmap` fenced JSON blocks for architecture and flow diagrams.
- Introduce every diagram in prose and explain what it shows so the page makes sense both visually and in plain text / `llms.txt`.
- Do not use decorative emojis in documentation prose or headings.

---

## 6. Metadata, Freshness, and Quality Gates

### 6.1 Page Header Metadata
Every chapter in `src/` begins with three HTML comment lines:

```markdown
<!-- description: Clear 110-160 character summary of the page used for SEO meta tags and LLM retrieval indices. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->
```

- `description`: Unique across the book, 110–160 characters, written as a complete sentence.
- `type`: One of the valid page types from Section 2.
- `last_verified` (or `generated_from: <version> on <YYYY-MM-DD>`): Date the page was last verified against the codebase.

### 6.2 Automated Verification
Documentation is tested like code. Every pull request must pass:
1. **Build (`bash build.sh`)**: Builds the mdBook, generates `book/llms-full.txt`, copies Markdown twins, and injects SEO/JSON-LD metadata via `seo.mjs`.
2. **Integrity & Structure Lint (`bash lint.sh`)**: Verifies internal links, anchor fragments, `{{#include}}` targets, JSON syntax, `llms.txt` coverage, and `structure_lint.py` rules.
3. **Prose Lint (`vale src`)**: Checks for marketing buzzwords, condescending ease words, British vs. American spelling consistency, and broken link text.
4. **Code Drift Guards (`cargo test --manifest-path drift/Cargo.toml`)**: Rust test suite verifying that documented configuration settings, defaults, `ORION_*` variables, task functions, JSONLogic operators, `clippy` rules, HTTP routes, error codes, audit actions, and Prometheus metrics match the Orion codebase.
