# Atlas test fixtures

Sample repositories for verifying and demonstrating Atlas (phases 0–7).

## Fixtures

| Folder | Size | Purpose |
|--------|------|---------|
| [`c_sample/`](c_sample/) | 3 C files | Minimal smoke test — fast, good for first `cargo run` |
| [`demo_app/`](demo_app/) | ~25 Python files | **Primary demo** — realistic fake backend with auth, users, payments, orders |
| [`ugly_app/`](ugly_app/) | ~32 Python files | **Stress test** — legacy folders, duplicate auth, dead routes, circular imports |

## Quick start (recommended)

```powershell
cd C:\Users\Rohan\Desktop\atlas
cargo build --release

# Scan the demo backend
.\target\release\atlas.exe scan tests/fixtures/demo_app --force

# See subsystems and entrypoints
.\target\release\atlas.exe architecture tests/fixtures/demo_app

# Ranked important files
.\target\release\atlas.exe top-files tests/fixtures/demo_app

# Trace the login feature across files
.\target\release\atlas.exe flow login tests/fixtures/demo_app

# Reading order for the auth subsystem
.\target\release\atlas.exe learn auth tests/fixtures/demo_app
```

## What each command should show on `demo_app`

- **architecture** — Subsystems: Auth, Users, Api, Payments, Orders (directory-based clustering)
- **top-files** — `main.py`, `auth/service.py`, `api/router.py` near the top
- **flow login** — `login_handler` → `login` → `get_by_email`, `verify_password`, `create_access_token`, …
- **learn auth** — Starts with `auth/routes.py`, then service/repository layers

## Ugly benchmark (`ugly_app`)

Run after flow or ranking changes — catches wrong call resolution and legacy noise:

```powershell
.\target\release\atlas.exe scan tests/fixtures/ugly_app --force
.\target\release\atlas.exe flow login tests/fixtures/ugly_app
.\target\release\atlas.exe explain auth tests/fixtures/ugly_app
```

See [`ugly_app/README.md`](ugly_app/README.md) for pass criteria.

## Minimal C fixture

```powershell
.\target\release\atlas.exe scan tests/fixtures/c_sample --force
.\target\release\atlas.exe flow core tests/fixtures/c_sample
```

See [`demo_app/README.md`](demo_app/README.md) for the file map and design intent.
