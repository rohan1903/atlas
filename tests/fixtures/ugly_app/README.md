# Ugly App — stress-test fixture

Deliberately messy repository for Atlas v1 benchmarks. Not runnable — structured to mimic a half-migrated internal backend.

## Intentional mess

- `legacy/`, `legacy_v2/`, `legacy_final/`, `legacy_final_fixed/` — competing auth implementations (dead code)
- Multiple `login_handler` and `AuthService.login` definitions
- `temp/auth_routes_backup.py` — dead routes never wired from `main.py`
- `services/unused_auth_service.py` — never imported
- Circular imports between `legacy/` and `legacy_final_fixed/`
- Feature flag `USE_NEW_AUTH` — only `auth/` is wired when true

## Canonical path (what Atlas should find)

Wired from `main.py` → `api/router.py` → `auth/routes.py` → `auth/service.py` → …

## Commands to run

```powershell
.\target\release\atlas.exe scan tests/fixtures/ugly_app --force
.\target\release\atlas.exe architecture tests/fixtures/ugly_app
.\target\release\atlas.exe top-files tests/fixtures/ugly_app
.\target\release\atlas.exe flow login tests/fixtures/ugly_app
.\target\release\atlas.exe learn auth tests/fixtures/ugly_app
.\target\release\atlas.exe explain auth tests/fixtures/ugly_app --no-llm
```

## Pass criteria

| Command | Expect |
|---------|--------|
| `architecture` | **Auth** subsystem leads; `auth/` files rank above legacy folders |
| `top-files` | `main.py`, `api/router.py`, `auth/service.py` above legacy/temp files |
| `flow login` | Canonical chain through `auth/routes.py`, not `legacy*` |
| `learn auth` | Starts with `auth/routes.py`, not legacy handlers |
| `explain auth` | Matches **Auth** subsystem; walkthrough uses canonical `auth/` path |
