# Real-repo benchmarks

Atlas is validated against cloned open-source repositories. These are not committed — clone locally before running.

## Starlette (Python ASGI framework)

```powershell
cd C:\Users\Rohan\Desktop\atlas
git clone --depth 1 https://github.com/encode/starlette.git tests/benchmarks/starlette

cargo build --release
.\target\release\atlas.exe scan tests/benchmarks/starlette --force
.\target\release\atlas.exe architecture tests/benchmarks/starlette
.\target\release\atlas.exe top-files tests/benchmarks/starlette --limit 15
.\target\release\atlas.exe flow routing tests/benchmarks/starlette
.\target\release\atlas.exe learn middleware tests/benchmarks/starlette
.\target\release\atlas.exe learn starlette tests/benchmarks/starlette
.\target\release\atlas.exe explain middleware tests/benchmarks/starlette --no-llm
.\target\release\atlas.exe explain routing tests/benchmarks/starlette --no-llm
```

### Expected after tuning (production code first)

| Command | What to expect |
|---------|----------------|
| `architecture` | **Starlette** subsystem leads; tests/metadata excluded from clustering |
| `top-files` | `starlette/routing.py`, `starlette/responses.py`, `starlette/requests.py` above test files |
| `flow routing` | Seeds from `starlette/routing.py`, not `tests/test_routing.py` |
| `learn middleware` | **Starlette / Middleware** reading path (`starlette/middleware/*.py`) |
| `explain middleware` | Multi-paragraph overview + snippets from `starlette/middleware/*.py` |
| `explain routing` | Flow-based explanation with call chain and snippets from `starlette/routing.py` |

### Scan stats (typical)

- ~120 files inventoried, ~67 parsed (Python + some HTML/JS in docs)
- ~2300 definitions, ~8700 graph edges
- Tests still scanned but deprioritized in rankings

## Demo fixture (synthetic)

For a controlled walkthrough, use `tests/fixtures/demo_app` (~27 files). See [../fixtures/README.md](../fixtures/README.md).

## Run your own repo

```powershell
.\target\release\atlas.exe scan C:\path\to\your-repo --force
.\target\release\atlas.exe architecture C:\path\to\your-repo
```

Paste output into Cursor if rankings or flows look wrong — that drives the next tuning pass.
