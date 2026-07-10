# Benchmark reports (automated)

Clone OSS repos, run Atlas, get one markdown report per repo. Paste reports into Cursor for review.

## Quick start

```powershell
cd C:\Users\Rohan\Desktop\atlas
cargo install --path .

# Clone all OSS repos in repos.json + generate reports
.\tests\benchmarks\run-reports.ps1 -Clone

# Or: only repos you already cloned
.\tests\benchmarks\run-reports.ps1 -Repo starlette,httpx

# Bundled fixtures only (no clone)
.\tests\benchmarks\run-reports.ps1 -Repo demo_app,ugly_app
```

Reports land in **`tests/benchmarks/reports/`** (e.g. `starlette.md`). Each file has full command output plus a verdict table for you to fill in.

## Add your own repo

Edit `repos.json`:

```json
{
  "id": "my-vms",
  "path": "C:/Users/Rohan/Desktop/Projects/visitor-management-system",
  "flow": "checkin",
  "learn": "registration",
  "explain": "registration"
}
```

Then:

```powershell
.\tests\benchmarks\run-reports.ps1 -Repo my-vms
```

## What gets run (per repo)

Same as `tests/VALIDATION.md`:

1. `atlas scan <repo> --force`
2. `atlas architecture <repo>`
3. `atlas top-files <repo> --limit 20`
4. `atlas flow <topic> <repo>`
5. `atlas learn <topic> <repo>`
6. `atlas explain <topic> <repo>`

Topics per repo are in `repos.json`.

## Sending results for review

1. Run the script.
2. Open `tests/benchmarks/reports/<repo>.md`.
3. Fill the **verdict** table (pass / partial / fail).
4. Paste the file into Cursor (or attach several `.md` files).

Cloned repos live in `tests/benchmarks/repos/` (not committed). Suggested OSS list is in `repos.json`.
