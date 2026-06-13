# Atlas build roadmap

Track progress here. Each phase is small enough to verify in one sitting. **Do not skip phases.**

---

## Current status

| Field | Value |
|-------|-------|
| **Active phase** | v1 complete — ugly benchmark next |
| **Milestone 1** | **Complete** |
| **v1 feature set** | **Complete** (`--no-llm` explain) |
| **Last updated** | 2026-06-14 |

### Progress overview

| Phase | Name | Status | Verified |
|-------|------|--------|----------|
| 0 | Project setup | Done | Yes |
| 1 | Repository scanner | Done | Yes |
| 2 | Tree-sitter parsing | Done | Yes |
| 3 | Graph and ranking | Done | Yes |
| 4 | `architecture` command | Done | Yes |
| 5 | MVP polish (`top-files`) | Done | Yes |
| 6 | Flow extraction | Done | Yes |
| 7 | LLM explanations | Done (7b; Ollama deferred to v1.1) | Yes |

**Milestone 1 complete when:** Phases 0–5 are done and verified on at least one benchmark repo.

---

## How to use this document

1. Tell Cursor **"Start Phase N"** when you are ready for that phase only.
2. As tasks finish, check boxes: `- [ ]` → `- [x]`.
3. Run the **How to verify** commands yourself and compare to **Expected output**.
4. If something fails, use **If it breaks** and the [debugging guide](#debugging-guide) below.
5. Update **Current status** at the top when a phase completes.

---

## Phase 0 — Project setup

**Goal:** Empty Rust CLI with stub commands — no real analysis yet.

### Tasks

- [x] Install Rust toolchain (see [README.md](README.md#install-rust-on-windows-one-time-manual))
- [x] `cargo init` and project metadata (`Cargo.toml`)
- [x] CLI with [clap](https://docs.rs/clap/): `scan`, `architecture`, `top-files`
- [x] Stub implementations print a clear "not implemented yet" message
- [x] `.gitignore` includes `target/`, `.atlas/`

### How to verify

```powershell
cd C:\Users\Rohan\Desktop\atlas
cargo run -- --help
cargo run -- scan .
cargo run -- architecture
cargo run -- top-files
```

**Expected output:**

- `--help` lists `scan`, `architecture`, `top-files`
- Each command runs without crash and reports not implemented (or similar)

### If it breaks

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `cargo` not recognized | Rust not installed or terminal not restarted | Install rustup, reopen terminal |
| Compile errors | dependency or syntax issue | Paste full error into Cursor |
| Very slow first build | Downloading crates | Wait; only happens once |

---

## Phase 1 — Repository scanner

**Goal:** Walk a repo, respect ignores, produce a file inventory under `.atlas/`.

### Tasks

- [x] Walk filesystem from user-provided path (default: `.`)
- [x] Respect `.gitignore` (use `ignore` crate)
- [x] Skip `node_modules`, `vendor`, build artifacts, binaries
- [x] Write inventory to `.atlas/` (JSON or DB — DB unified in Phase 3 is fine)
- [x] Print summary: total files, skipped, errors
- [x] Add `--verbose` flag for skip reasons

### How to verify

```powershell
cargo run -- scan C:\path\to\small-repo
```

**Expected output:**

- Creates `.atlas/` in the scanned repo
- Prints file count > 0 for a normal code project
- Does **not** traverse `node_modules` (check with `--verbose`)

### If it breaks

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| 0 files found | Wrong path or over-aggressive ignores | Pass explicit path; use `--verbose` |
| Permission denied | System/protected folder | Scan a project folder you own |
| Scan takes forever | Traversing huge ignored dirs | Bug — report to Cursor with `--verbose` output |

---

## Phase 2 — Tree-sitter parsing

**Goal:** Extract imports, definitions, and calls from Python, TS, JS, Go, and **C** files (`.c`, `.h`).

### Tasks

- [x] Integrate Tree-sitter + language grammars (Python, TypeScript, JavaScript, Go, C)
- [x] Per file: imports, functions, classes, call expressions (best-effort)
- [x] Store symbols during scan (`.atlas/symbols.json`)
- [x] Report parse stats: parsed, unsupported extension, failed parse
- [x] Single file parse failure must not abort full scan

### How to verify

```powershell
cargo run -- scan C:\path\to\python-or-node-repo
```

**Expected output:**

- Scan completes with parsed file count > 0
- `.atlas/` contains symbol/parse data
- Unsupported extensions listed in summary, not silent

### If it breaks

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| All files "unsupported" | Wrong extensions or grammar not loaded | Paste scan summary into Cursor |
| Crash on one file | Should be caught — file a bug | Note file path from error |
| Empty call graph | Normal for dynamic code | Documented limitation; continue to Phase 3 |

---

## Phase 3 — Graph and importance ranking

**Goal:** SQLite graph + `atlas top-files` with structural importance scores.

### Tasks

- [x] SQLite schema: `nodes`, `edges`, `file_scores`
- [x] Node types (MVP): File, Function, Class
- [x] Edge types (MVP): IMPORTS, CALLS, DEFINES
- [x] Ranking: inbound references + entrypoint heuristics (`main.py`, `index.ts`, `main.go`, etc.)
- [x] Implement `atlas top-files`
- [x] Require prior scan; clear error if `.atlas/` missing

### How to verify

```powershell
cargo run -- scan C:\path\to\benchmark-repo
cargo run -- top-files
```

**Expected output:**

- Ranked list of files with scores
- Entrypoints and highly imported files rank near the top
- `constants.py`-style leaf files rank lower

### Benchmark repos (pick at least one)

- Small FastAPI or Flask app
- Small Express or Next.js app
- This Atlas repo (once it has `src/`)

### If it breaks

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| No atlas data | Forgot scan | Run `scan` first |
| All scores 0 | No edges from parsing | Rescan with `--verbose`; check Phase 2 |
| Surprising order | Heuristics need tuning | Note repo + output; tune in Phase 5 |

---

## Phase 4 — `architecture` command

**Goal:** Subsystems, entrypoints, and critical files — no LLM.

### Tasks

- [x] Detect entrypoints (filename heuristics + graph signals)
- [x] Detect subsystems (directory clusters + import density)
- [x] Implement `atlas architecture`
- [x] Output: repo name, subsystems, entrypoints, critical files

### How to verify

```powershell
cargo run -- scan C:\path\to\benchmark-repo
cargo run -- architecture
```

**Expected output:**

- Named subsystems (e.g. Authentication, API, Database) — rough match to folder layout
- Entrypoints listed (`main.py`, `api/router.ts`, etc.)
- Critical files overlap with `top-files` leaders

**PRD check:** architecture understandable in under ~5 minutes of reading output.

### If it breaks

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| One giant subsystem | Flat repo structure | Expected fallback: top-level dirs |
| Too many subsystems | Threshold too low | Tune clustering in Cursor session |
| Empty subsystems | No parse data | Revisit Phase 2–3 |

---

## Phase 5 — MVP polish (Milestone 1)

**Goal:** Reliable, readable CLI — **Milestone 1 complete.**

### Tasks

- [x] Progress feedback during `scan`
- [x] `atlas scan --force` to rebuild `.atlas/`
- [x] Clean terminal formatting for all three commands
- [x] End-of-scan summary (files, symbols, edges, errors)
- [x] README quick start verified end-to-end
- [x] Update this roadmap: Phases 0–5 marked Done

### How to verify

```powershell
cargo build --release
.\target\release\atlas.exe scan tests/fixtures/c_sample --force
.\target\release\atlas.exe architecture tests/fixtures/c_sample
.\target\release\atlas.exe top-files tests/fixtures/c_sample
```

**Milestone 1 checklist:**

- [x] Architecture identifiable in under 5 minutes
- [x] Important files found in under 2 minutes
- [x] Zero LLM API calls
- [x] Works on benchmark repo (`tests/fixtures/c_sample`)

### If it breaks

Use the [debugging guide](#debugging-guide) below.

---

## Phase 6 — Flow extraction (post-MVP)

**Goal:** `atlas flow <name>` traces execution paths on the call graph.

### Tasks

- [x] Seed search from route/function name
- [x] Traverse CALLS edges with depth limit
- [x] Framework route detection (incremental: FastAPI, Express, Gin)
- [x] Implement `atlas flow` and `atlas learn`

### How to verify

```powershell
cargo run -- scan tests/fixtures/c_sample --force
cargo run -- flow core tests/fixtures/c_sample
cargo run -- learn include tests/fixtures/c_sample
```

**Expected output:** Plausible chain, e.g. `core_init → helper` and a reading order for the matched subsystem.

---

## Phase 7 — LLM explanations (post-MVP)

**Goal:** `atlas explain` narrates graph evidence — never invents structure.

### Tasks

- [x] Gather graph slice + file snippets for topic *(7a: graph slice; 7b: snippets)*
- [x] Optional local LLM (Ollama) or API — user-configured *(deferred to v1.1)*
- [x] Citations always include real file paths from graph *(7a)*
- [x] `--no-llm` template fallback *(7a; 7b: multi-paragraph overview + snippets)*
- [x] Implement `atlas explain` *(7a: template mode; 7b: richer output)*

### How to verify

```powershell
cargo build --release
.\target\release\atlas.exe scan tests/benchmarks/starlette --force
.\target\release\atlas.exe explain middleware tests/benchmarks/starlette --no-llm
.\target\release\atlas.exe explain routing tests/benchmarks/starlette --no-llm
cargo test
```

**Expected output:** Multi-paragraph overview, graph citations with line anchors, and code snippets from real files. No hallucinated paths.

---

## v1 — feature complete

Ship when these commands are stable on benchmarks:

```powershell
cargo build --release
.\target\release\atlas.exe scan tests/fixtures/demo_app --force
.\target\release\atlas.exe scan tests/fixtures/ugly_app --force
.\target\release\atlas.exe scan tests/benchmarks/starlette --force
.\target\release\atlas.exe explain auth tests/fixtures/demo_app --no-llm
.\target\release\atlas.exe explain auth tests/fixtures/ugly_app --no-llm
.\target\release\atlas.exe explain middleware tests/benchmarks/starlette --no-llm
cargo test
```

**v1 commands:** `scan`, `architecture`, `top-files`, `flow`, `learn`, `explain --no-llm`

**Deferred to v1.1:** Ollama/API narration (`explain --llm`)

---

## Ugly benchmark (`tests/fixtures/ugly_app`)

Stress-test fixture with legacy folders, duplicate handlers, dead routes, circular imports, and unused services. Run after any flow/ranking change:

```powershell
.\target\release\atlas.exe scan tests/fixtures/ugly_app --force
.\target\release\atlas.exe flow login tests/fixtures/ugly_app
.\target\release\atlas.exe explain auth tests/fixtures/ugly_app --no-llm
```

**Pass:** canonical `auth/routes.py` → `auth/service.py` chain; legacy handlers not in flow or walkthrough.

---

## v1.1 backlog (do not start until user says so)

1. Behavior tracing (framework wiring, request journeys)
2. Confidence scoring on inferred vs traced claims
3. `atlas impact <file>`
4. Rust grammar support
5. LLM narration (`explain --llm`)

---

## Debugging guide

When anything fails:

1. **Note the phase** from the top of this file.
2. **Copy the full terminal output** (not just the last line).
3. **Paste into Cursor** with: what you ran, what you expected, benchmark repo if any.
4. **Delete stale cache:** remove `.atlas/` in the target repo, run `scan` again.
5. **Shrink the problem:** try a smaller repo with few files.
6. **Use `--verbose`** when available.

### Quick symptom table

| Symptom | Likely cause | What to do |
|---------|--------------|------------|
| Command not found | Binary not built | `cargo build --release` |
| No atlas data | Never scanned | `atlas scan .` first |
| Stale/wrong results | Code changed since scan | `atlas scan --force .` |
| Weird rankings | Approximate graph | Expected early; tune in Phase 5 |
| Rust errors | Toolchain issue | `rustc --version`; reinstall rustup |

---

## Glossary

| Term | Meaning |
|------|---------|
| **SQLite** | A small database stored as one file on disk (`.atlas/graph.db`). No server. |
| **Tree-sitter** | Tool that reads code and outputs structure (functions, imports, calls). |
| **Graph** | Map of **nodes** (files, functions) and **edges** (imports, calls). |
| **Centrality** | How "in the middle" a node is — many connections → often more important. |
| **Scan** | One-time analysis pass that builds/refreshes `.atlas/`. |
| **Phase** | A chunk of work with a verify step — do not skip. |

---

## What you do vs what Cursor does

| You | Cursor |
|-----|--------|
| Say "Start Phase N" | Implements only that phase |
| Run verify commands | Writes Rust code and fixes errors |
| Check boxes in this file | Updates roadmap when asked |
| Paste errors when stuck | Debugs from terminal output |

**Next step:** Install Rust if needed, then say **"Start Phase 0"**.
