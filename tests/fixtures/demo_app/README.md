# Demo App — Atlas benchmark fixture



A realistic **fake backend** (~30 Python files) designed to show how Atlas works end-to-end.



It mimics a web API with **auth**, **users**, **payments**, **orders**, and **api** layers — not a runnable app, but structured like one.



## What Atlas should find here



| Command | What to expect |

|---------|----------------|

| `atlas architecture` | Subsystems: **Auth**, **Users**, **Orders**, **Api**, **Payments**, … |

| `atlas top-files` | `main.py`, `auth/service.py`, `api/router.py`, `orders/service.py` near the top |

| `atlas flow login` | `login_handler` → `login` → `get_by_email` → `verify_password` → `create_access_token` → `record_login` |

| `atlas flow place_order` | `create_order_handler` → `place_order` → payment + repository chain |

| `atlas learn auth` | Starts with `auth/routes.py`, then service/repository layers |



## How to scan



```powershell

cd C:\Users\Rohan\Desktop\atlas

cargo build --release



.\target\release\atlas.exe scan tests/fixtures/demo_app --force

.\target\release\atlas.exe architecture tests/fixtures/demo_app

.\target\release\atlas.exe top-files tests/fixtures/demo_app

.\target\release\atlas.exe flow login tests/fixtures/demo_app

.\target\release\atlas.exe flow place_order tests/fixtures/demo_app

.\target\release\atlas.exe learn auth tests/fixtures/demo_app

.\target\release\atlas.exe learn orders tests/fixtures/demo_app

```



## File map



```text

main.py                 → app entrypoint

api/router.py           → wires routes (login, users, orders)

auth/

  routes.py             → POST /login handler

  service.py            → AuthService (core auth logic)

  repository.py         → credential storage

  token.py              → JWT helpers

users/

  service.py            → profile logic

  repository.py         → user DB access

  models.py             → data shapes

orders/

  routes.py             → POST /orders handler

  service.py            → OrderService (crosses payments + users)

  repository.py         → order persistence

payments/

  service.py            → charge cards

  gateway.py            → external payment API

notifications/

  email.py              → leaf utility (low rank)

utils/logger.py         → shared logging

config/settings.py      → configuration

```



## Intentional design



- **auth/service.py** and **orders/service.py** are hubs — imported from many places → rank high

- **login flow** crosses 4+ files → good `atlas flow` demo

- **place_order flow** crosses orders → payments → users → another flow demo

- **notifications/email.py** is a leaf — should rank low

- **README.md** is documentation only — should not appear as an entrypoint



## Compare with the minimal fixture



For a 3-file smoke test, use `tests/fixtures/c_sample` (C). See [../README.md](../README.md).

