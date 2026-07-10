# Generate Atlas benchmark reports for repos listed in repos.json.
#
# Usage:
#   .\tests\benchmarks\run-reports.ps1 -Clone
#   .\tests\benchmarks\run-reports.ps1 -Repo starlette
#   .\tests\benchmarks\run-reports.ps1 -Repo demo_app,httpx
#   .\tests\benchmarks\run-reports.ps1 -CloneOnly
#
# Reports: tests/benchmarks/reports/<id>.md

param(
    [switch] $Clone,
    [switch] $CloneOnly,
    [string[]] $Repo = @()
)

$ErrorActionPreference = "Stop"

$AtlasRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $AtlasRoot

$ManifestPath = Join-Path $PSScriptRoot "repos.json"
if (-not (Test-Path $ManifestPath)) {
    throw "Missing manifest: $ManifestPath"
}

$Manifest = Get-Content $ManifestPath -Raw | ConvertFrom-Json
$CloneRoot = Join-Path $AtlasRoot ($Manifest.clone_dir -replace "/", "\")
$ReportsRoot = Join-Path $AtlasRoot ($Manifest.reports_dir -replace "/", "\")
New-Item -ItemType Directory -Force -Path $CloneRoot, $ReportsRoot | Out-Null

function Get-RepoPath($Entry) {
    if ($Entry.path) {
        return (Resolve-Path (Join-Path $AtlasRoot ($Entry.path -replace "/", "\")) -ErrorAction SilentlyContinue)
    }
    $dir = Join-Path $CloneRoot $Entry.id
    if (Test-Path $dir) {
        return (Resolve-Path $dir)
    }
    return $null
}

function Get-GitHead($RepoPath) {
    Push-Location $RepoPath
    try {
        $head = git rev-parse --short HEAD 2>$null
        if ($LASTEXITCODE -eq 0) { return $head.Trim() }
    } finally {
        Pop-Location
    }
    return "unknown"
}

function Invoke-AtlasSection {
    param(
        [string] $Title,
        [string[]] $AtlasArgs
    )

    $commandLine = "atlas $($AtlasArgs -join ' ')"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = (& atlas @AtlasArgs 2>&1 | ForEach-Object { "$_" }) -join "`n"
        $exit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prev
    }

    $lines = @(
        "## $Title"
        ""
        '```powershell'
        $commandLine
        '```'
        ""
        "exit code: **$exit**"
        ""
        '```text'
        $output.TrimEnd()
        '```'
        ""
    )

    return @{
        Markdown = ($lines -join "`n")
        ExitCode = $exit
    }
}

if (-not (Get-Command atlas -ErrorAction SilentlyContinue)) {
    throw "atlas not on PATH. From repo root: cargo install --path ."
}

$Entries = @($Manifest.repos)
if ($Repo.Count -gt 0) {
    $wanted = $Repo | ForEach-Object { $_.Trim().ToLower() }
    $Entries = $Entries | Where-Object { $wanted -contains $_.id.ToLower() }
    if ($Entries.Count -eq 0) {
        throw "No matching repos in manifest. IDs: $($Manifest.repos.id -join ', ')"
    }
}

foreach ($entry in $Entries) {
    $repoPath = Get-RepoPath $entry

    if (-not $repoPath -and $entry.url -and ($Clone -or $CloneOnly)) {
        $dest = Join-Path $CloneRoot $entry.id
        Write-Host "Cloning $($entry.id) ..." -ForegroundColor Cyan
        git clone --depth 1 $entry.url $dest
        if ($LASTEXITCODE -ne 0) { throw "git clone failed for $($entry.id)" }
        $repoPath = Resolve-Path $dest
    }

    if ($CloneOnly) { continue }

    if (-not $repoPath) {
        Write-Host "SKIP $($entry.id) - not found. Run: .\tests\benchmarks\run-reports.ps1 -Clone -Repo $($entry.id)" -ForegroundColor Yellow
        continue
    }

    Write-Host ("Reporting {0} -> {1}" -f $entry.id, $repoPath) -ForegroundColor Green

    $head = Get-GitHead $repoPath
    $url = if ($entry.url) { $entry.url } else { "(local: $($entry.path))" }
    $date = Get-Date -Format "yyyy-MM-dd HH:mm"

    $sections = New-Object System.Collections.Generic.List[string]
    $failures = 0

    $runs = @(
        @{ Title = "scan"; Args = @("scan", $repoPath, "--force") },
        @{ Title = "architecture"; Args = @("architecture", $repoPath) },
        @{ Title = "top-files"; Args = @("top-files", $repoPath, "--limit", "20") },
        @{ Title = "flow $($entry.flow)"; Args = @("flow", $entry.flow, $repoPath) },
        @{ Title = "learn $($entry.learn)"; Args = @("learn", $entry.learn, $repoPath) },
        @{ Title = "explain $($entry.explain)"; Args = @("explain", $entry.explain, $repoPath) }
    )

    foreach ($run in $runs) {
        $result = Invoke-AtlasSection -Title $run.Title -AtlasArgs $run.Args
        [void]$sections.Add($result.Markdown)
        if ($result.ExitCode -ne 0) { $failures++ }
    }

    $header = @"
# Atlas report: $($entry.id)

| Field | Value |
|-------|-------|
| URL | $url |
| Path | $repoPath |
| Commit | $head |
| Generated | $date |
| Flow topic | $($entry.flow) |
| Learn topic | $($entry.learn) |
| Explain topic | $($entry.explain) |
| Failed commands | $failures / $($runs.Count) |

## Your verdict (fill in before sending for review)

| Command | pass / partial / fail | Notes |
|---------|-------------------------|-------|
| architecture | | |
| top-files | | |
| flow | | |
| learn | | |
| explain | | |

## Overall notes

_What looked wrong or surprisingly right (1-3 sentences)._

---

"@

    $report = $header + ($sections -join "`n")
    $outFile = Join-Path $ReportsRoot "$($entry.id).md"
    $utf8Bom = New-Object System.Text.UTF8Encoding $true
    [System.IO.File]::WriteAllText($outFile, $report, $utf8Bom)
    Write-Host "  -> $outFile" -ForegroundColor DarkGray
}

Write-Host ""
Write-Host "Done. Reports in: $ReportsRoot" -ForegroundColor Green
Write-Host "Share a .md file in Cursor chat for review."
