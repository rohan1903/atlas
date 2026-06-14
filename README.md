<div align="center">

# Atlas

**Deterministic repository intelligence for onboarding — not an AI coding assistant.**

Understand architecture, important files, and where to start reading — from a static call graph, not an LLM guess.

<br/>

[![Release](https://img.shields.io/github/v/release/rohan1903/atlas?style=for-the-badge&label=v1.0.0)](https://github.com/rohan1903/atlas/releases/tag/v1.0.0)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey?style=for-the-badge)](#installation)
[![Local first](https://img.shields.io/badge/Cloud-not%20required-success?style=for-the-badge)](#local-cache-atlas)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)

<br/>

[Installation](#installation) ·
[Quick start](#quick-start) ·
[Commands](#commands) ·
[Example output](#example-output) ·
[Roadmap](ROADMAP.md)

</div>

---

## Why Atlas

When you join an unfamiliar codebase, you ask the same questions:

- What are the major parts of this system?
- Which files should I read first?
- How does a feature like *auth* or *check-in* flow through the code?

Atlas answers those from **structure** — scans, parses, builds a graph, ranks files, traces flows — without calling an LLM. Explanations in v1 use graph evidence and code snippets (`explain --no-llm`). Optional LLM narration is planned for v1.1.

| Atlas **is** | Atlas **is not** |
|--------------|------------------|
| An onboarding map for large repos | A code generator or IDE replacement |
| Local-first (`.atlas/` on disk) | A cloud analysis service |
| Deterministic graph + heuristics | An autonomous agent |
| Honest about approximate graphs | A source of invented file paths |

---

## Installation

**Requirements:** [Rust](https://rustup.rs/) 1.70+ (`rustc`, `cargo`), Git (optional).

```powershell
git clone https://github.com/rohan1903/atlas.git
cd atlas
cargo build --release
```

Binary: `target/release/atlas.exe` (Windows) or `target/release/atlas` (Linux/macOS).

<details>
<summary><strong>First-time Rust on Windows</strong></summary>

1. Install from [rustup.rs](https://rustup.rs/) (default options).
2. Restart your terminal.
3. Verify: `rustc --version` and `cargo --version`.

The first `cargo build` downloads dependencies and may take several minutes.

</details>

---

## Quick start

Point Atlas at any repository. It writes a cache under `.atlas/` in that repo.

```powershell
# Build once
cargo build --release

# Scan a project (use your own path or the bundled fixtures)
.\target\release\atlas.exe scan tests/fixtures/demo_app --force

# Orientation
.\target\release\atlas.exe architecture tests/fixtures/demo_app
.\target\release\atlas.exe top-files tests/fixtures/demo_app

# Feature-level reading
.\target\release\atlas.exe flow login tests/fixtures/demo_app
.\target\release\atlas.exe learn auth tests/fixtures/demo_app
.\target\release\atlas.exe explain auth tests/fixtures/demo_app --no-llm
```

**Linux / macOS:** replace `.\target\release\atlas.exe` with `./target/release/atlas`.

Fixtures: [demo_app](tests/fixtures/demo_app) (clean), [ugly_app](tests/fixtures/ugly_app) (stress test). Real-repo benchmark: [Starlette](tests/benchmarks/README.md).

---

## Commands

| Command | Purpose |
|---------|---------|
| `atlas scan [path]` | Inventory + parse + graph → `.atlas/` |
| `atlas scan --force` | Rebuild cache from scratch |
| `atlas architecture [path]` | Subsystems, entrypoints, critical files |
| `atlas top-files [path]` | Ranked **code files** (tests/docs excluded by default) |
| `atlas top-files --include-tests` | Include test files |
| `atlas top-files --include-metadata` | Include README, config, requirements, etc. |
| `atlas flow <name> [path]` | Compressed primary execution path |
| `atlas flow <name> --verbose` | Full call-graph trace |
| `atlas learn <topic> [path]` | Reading order for a subsystem |
| `atlas explain <topic> [path] --no-llm` | Overview, walkthrough, citations, snippets |

Global: `--color` forces syntax highlighting; `NO_COLOR=1` disables colors.

---

## Example output

**Architecture** — subsystems and entrypoints without reading every folder:

```text
Subsystems
  1. Auth (5 files, score 42, internal links 3)
     top: auth/routes.py, auth/service.py, auth/repository.py
  2. Orders (4 files, ...)

Entrypoints
  - main.py
  - api/router.py
```

**Flow** — primary path (full graph with `--verbose`):

```text
Flow: login
  seed login_handler

  login_handler  →  login  →  get_by_email  →  verify_password  →  create_access_token
```

**Explain** — graph-grounded reading order with real citations (v1 template mode):

```text
Topic: auth
Citations
  1. auth/routes.py @ login_handler:21
  2. auth/service.py @ login:16
Snippets
  (syntax-highlighted source from those files)
```

---

## How it works

```mermaid
flowchart LR
    scan["atlas scan"] --> parse[Tree-sitter]
    parse --> graph[(SQLite graph)]
    graph --> out[architecture · top-files · flow · explain]
    out --> llm["LLM narrator (v1.1)"]
```

1. **Scan** — walk the tree, respect `.gitignore`, skip `node_modules` and build artifacts.
2. **Parse** — Tree-sitter extracts imports, definitions, and call expressions (best-effort).
3. **Graph** — files, functions, and edges stored in `.atlas/graph.db`.
4. **Intelligence** — ranking, subsystem clustering, flow seeds, explain templates.
5. **Output** — terminal reports you can skim in minutes.

**Principle:** the graph is the product. LLM narration (v1.1) must cite graph evidence — never invent structure.

---

## Supported languages

| Language | Extensions | Status |
|----------|------------|--------|
| Python | `.py` | Supported |
| TypeScript / JavaScript | `.ts`, `.tsx`, `.js`, `.jsx` | Supported |
| Go | `.go` | Supported |
| C | `.c`, `.h` | Supported (approximate on large/kernel trees) |

**Planned (v1.1+):** Rust, Java, C#, C++, Kotlin.

Unsupported extensions are skipped with counts in the scan summary. Files over 5 MB are not parsed.

---

## Local cache: `.atlas/`

```text
.atlas/
  inventory.json   # scanned file list
  symbols.json     # parsed structure
  graph.db         # SQLite graph and scores
```

- Created in the **scanned** repository, not in the Atlas install directory.
- Safe to delete; run `atlas scan --force` to rebuild.
- Ignored by git (see `.gitignore`). Stays on your machine.

---

## Limitations (v1)

- Call graphs are **static** — dynamic dispatch, reflection, and framework magic may be missing.
- Flows show **function names**, not semantic steps like “validate token” (v1.1 behavior tracing).
- No confidence labels yet on inferred vs traced claims (v1.1).
- `explain` without `--no-llm` waits for v1.1 Ollama/API integration.

See [ROADMAP.md](ROADMAP.md) for the full v1.1 backlog.

---

## Project layout

```text
src/
  scan/           filesystem inventory
  parse/          tree-sitter extraction
  graph/          SQLite nodes, edges, ranking
  intelligence/   subsystems, flows, explain
  commands/       CLI presentation
tests/fixtures/   demo_app, ugly_app, c_sample
```

---

## Development

```powershell
cargo test
cargo build --release
```

Phased build history and verify steps: [ROADMAP.md](ROADMAP.md).

**Current release:** [v1.0.0](https://github.com/rohan1903/atlas/releases/tag/v1.0.0) — `scan`, `architecture`, `top-files`, `flow`, `learn`, `explain --no-llm`.

---

## Getting help

1. Run `atlas scan --force` on the target repo after code changes.
2. Copy the **full terminal output** and note which command failed.
3. Try a smaller repo or fixture to isolate the issue.
4. See the debugging guide in [ROADMAP.md](ROADMAP.md).

---

## License

[MIT](LICENSE) — Copyright (c) 2026 Rohan

---

<div align="center">

**Atlas v1.0.0** — graph-first onboarding for code you did not write.

</div>
