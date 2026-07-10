use crate::graph;
use crate::parse;
use crate::progress;
use crate::style;
use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

const INVENTORY_FILE: &str = "inventory.json";
const ATLAS_DIR: &str = ".atlas";
const LIST_LIMIT: usize = 50;
const SKIP_DIR_NAMES: &[&str] = &[
    ".atlas",
    ".git",
    ".hg",
    ".svn",
    ".cursor",
    ".vscode",
    ".idea",
    ".codex",
    ".agents",
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".next",
    ".nuxt",
    ".turbo",
    ".venv",
    "venv",
    ".tox",
    "coverage",
    "htmlcov",
];

/// Extensions treated as non-source binaries or assets.
const SKIP_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "a", "lib", "o", "obj", "pdb", "class", "jar", "war", "ear",
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "png", "jpg", "jpeg", "gif", "webp", "ico",
    "bmp", "svg", "mp3", "mp4", "wav", "avi", "mov", "mkv", "woff", "woff2", "ttf", "eot", "pdf",
    "doc", "docx", "xls", "xlsx", "ppt", "pptx", "bin", "dat", "db", "sqlite", "sqlite3", "lock",
    "min.js", "min.css",
];

#[derive(Debug, Serialize, Clone)]
pub struct InventoryFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
struct ScanSummary {
    files: usize,
    skipped: usize,
    errors: usize,
}

#[derive(Debug, Serialize)]
struct Inventory {
    version: u32,
    scanned_at: String,
    root: String,
    files: Vec<InventoryFile>,
    summary: ScanSummary,
}

#[derive(Debug, Default)]
struct ScanStats {
    files: Vec<InventoryFile>,
    skipped: usize,
    errors: usize,
}

pub fn run(path: &Path, verbose: bool, list: bool, force: bool) -> Result<(), String> {
    let root = path
        .canonicalize()
        .map_err(|e| format!("could not resolve path {}: {e}", path.display()))?;

    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }

    let atlas_dir = root.join(ATLAS_DIR);
    if force && atlas_dir.exists() {
        fs::remove_dir_all(&atlas_dir)
            .map_err(|e| format!("could not remove {}: {e}", atlas_dir.display()))?;
        if verbose {
            progress::detail(&format!("removed stale {}", atlas_dir.display()));
        }
    }

    progress::step("Inventorying files");
    if verbose {
        progress::detail(&format!("root: {}", root.display()));
        progress::detail("rules: .gitignore + built-in skip lists");
    }

    let stats = walk_repository(&root, verbose)?;
    progress::done(&format!("{} files inventoried", stats.files.len()));

    fs::create_dir_all(&atlas_dir)
        .map_err(|e| format!("could not create {}: {e}", atlas_dir.display()))?;

    let inventory = Inventory {
        version: 1,
        scanned_at: iso_timestamp(),
        root: root.display().to_string(),
        summary: ScanSummary {
            files: stats.files.len(),
            skipped: stats.skipped,
            errors: stats.errors,
        },
        files: stats.files,
    };

    let inventory_path = atlas_dir.join(INVENTORY_FILE);
    let json = serde_json::to_string_pretty(&inventory)
        .map_err(|e| format!("could not serialize inventory: {e}"))?;
    fs::write(&inventory_path, json)
        .map_err(|e| format!("could not write {}: {e}", inventory_path.display()))?;

    progress::step("Parsing symbols");
    if verbose {
        progress::detail(&format!(
            "languages: {}",
            parse::supported_languages_label()
        ));
    }

    let inventory_paths: Vec<String> = inventory.files.iter().map(|f| f.path.clone()).collect();
    let parse_output = parse::parse_inventory(&root, &inventory_paths, verbose)?;
    progress::done(&format!("{} files parsed", parse_output.summary.parsed));

    progress::step("Building graph");
    let symbols_path = parse::write_symbols(&atlas_dir, &parse_output)?;
    let (graph_path, graph_stats) =
        graph::build_and_store(&atlas_dir, &inventory.files, &parse_output)?;
    progress::done(&format!(
        "{} nodes, {} edges",
        graph_stats.nodes, graph_stats.edges
    ));

    let symbol_totals = parse::symbol_totals(&parse_output);
    let repo_name = root.file_name().unwrap_or_default().to_string_lossy();
    let repo_hint = if path == Path::new(".") {
        ".".to_string()
    } else {
        path.display().to_string()
    };

    println!();
    println!(
        "{} {}",
        style::label("Repository:"),
        style::emphasis(&repo_name)
    );
    println!(
        "  {} {}",
        style::muted("inventory"),
        style::path(&inventory_path.display().to_string())
    );
    println!(
        "  {} {}",
        style::muted("symbols"),
        style::path(&symbols_path.display().to_string())
    );
    println!(
        "  {} {}",
        style::muted("graph"),
        style::path(&graph_path.display().to_string())
    );

    println!();
    println!("{}", style::heading("Scan"));
    println!(
        "  {} {}",
        style::label("files:"),
        style::metric_value("files", inventory.summary.files)
    );
    println!(
        "  {} {}",
        style::label("skipped:"),
        style::metric_value("skipped", inventory.summary.skipped)
    );
    println!(
        "  {} {}",
        style::label("errors:"),
        style::metric_value("errors", inventory.summary.errors)
    );
    println!();
    println!("{}", style::heading("Parsing"));
    println!(
        "  {} {}",
        style::label("parsed:"),
        style::metric_value("files", parse_output.summary.parsed)
    );
    println!(
        "  {} {}",
        style::label("unsupported:"),
        style::metric_value("skipped", parse_output.summary.unsupported)
    );
    println!(
        "  {} {}",
        style::label("failed:"),
        style::metric_value("errors", parse_output.summary.failed)
    );
    println!(
        "  {} {}",
        style::label("too large:"),
        style::metric_value("skipped", parse_output.summary.too_large)
    );

    crate::commands::report::print_scan_complete(
        inventory.summary.files,
        &parse_output.summary,
        symbol_totals,
        graph_stats,
        inventory.summary.files,
    );

    if inventory.summary.files == 0 {
        eprintln!();
        eprintln!(
            "{}",
            style::warning("warning: no files inventoried — check path or run with --verbose")
        );
    } else if list {
        print_file_list(&inventory.files, &inventory_path);
    }

    crate::commands::report::print_next_steps(&repo_hint);

    Ok(())
}

fn print_file_list(files: &[InventoryFile], inventory_path: &Path) {
    println!();
    println!("{}", style::heading("Files"));

    let total = files.len();
    let shown = total.min(LIST_LIMIT);

    for file in &files[..shown] {
        println!("  {}", style::path(&file.path));
    }

    if total > LIST_LIMIT {
        let remaining = total - LIST_LIMIT;
        println!();
        println!(
            "{}",
            style::muted(&format!(
                "... and {remaining} more in {}",
                inventory_path.display()
            ))
        );
    }
}

fn walk_repository(root: &Path, verbose: bool) -> Result<ScanStats, String> {
    let root_buf = root.to_path_buf();
    let stats = Arc::new(Mutex::new(ScanStats::default()));

    let walker = WalkBuilder::new(&root_buf)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry({
            let root_buf = root_buf.clone();
            let stats = Arc::clone(&stats);
            move |entry| {
                let mut stats = stats.lock().expect("scan stats lock");
                !should_skip_entry(entry.path(), &root_buf, verbose, &mut stats)
            }
        })
        .build();

    for entry in walker {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let mut stats = stats.lock().expect("scan stats lock");

                if let Some(reason) = skip_file_reason(path) {
                    stats.skipped += 1;
                    if verbose {
                        eprintln!(
                            "{}",
                            style::skip_verbose(
                                "skip file:",
                                &display_relative(path, &root_buf),
                                reason,
                            )
                        );
                    }
                    continue;
                }

                match file_metadata(path, &root_buf) {
                    Ok(file) => {
                        stats.files.push(file);
                        progress::file_tick(stats.files.len(), "inventoried", verbose);
                    }
                    Err(error) => {
                        stats.errors += 1;
                        if verbose {
                            eprintln!(
                                "{}",
                                style::error_verbose(
                                    &path.display().to_string(),
                                    &error.to_string()
                                )
                            );
                        }
                    }
                }
            }
            Err(error) => {
                stats.lock().expect("scan stats lock").errors += 1;
                if verbose {
                    eprintln!("{}", style::error_verbose("walk", &error.to_string()));
                }
            }
        }
    }

    let mut stats = Arc::try_unwrap(stats)
        .expect("scan stats still in use")
        .into_inner()
        .expect("scan stats lock");
    stats.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(stats)
}

fn should_skip_entry(path: &Path, root: &Path, verbose: bool, stats: &mut ScanStats) -> bool {
    if path == root {
        return false;
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if SKIP_DIR_NAMES.contains(&name) {
            stats.skipped += 1;
            if verbose {
                eprintln!(
                    "{}",
                    style::skip_verbose(
                        "skip dir:",
                        &display_relative(path, root),
                        &format!("built-in rule: {name}"),
                    )
                );
            }
            return true;
        }
    }

    false
}

fn skip_file_reason(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_str()?.to_lowercase();

    for ext in SKIP_EXTENSIONS {
        if file_name.ends_with(&format!(".{ext}")) || file_name == *ext {
            return Some("binary or asset extension");
        }
    }

    if is_probably_binary(path) {
        return Some("binary content");
    }

    None
}

fn is_probably_binary(path: &Path) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let mut buffer = [0u8; 8192];
    let read = match file.read(&mut buffer) {
        Ok(0) => return false,
        Ok(n) => n,
        Err(_) => return false,
    };

    buffer[..read].contains(&0)
}

fn file_metadata(path: &Path, root: &Path) -> io::Result<InventoryFile> {
    let metadata = fs::metadata(path)?;
    Ok(InventoryFile {
        path: display_relative(path, root),
        size_bytes: metadata.len(),
    })
}

fn display_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(_)))
        .collect::<PathBuf>()
        .display()
        .to_string()
        .replace('\\', "/")
}

fn iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("unix:{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("atlas-scan-test-{name}-{nanos}"))
    }

    #[test]
    fn skips_node_modules_even_without_gitignore() {
        let root = temp_root("node-modules");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            root.join("node_modules/pkg/index.js"),
            "module.exports = {}",
        )
        .unwrap();

        let stats = walk_repository(&root, false).unwrap();

        let paths: Vec<_> = stats.files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"src/main.rs"));
        assert!(!paths.iter().any(|p| p.contains("node_modules")));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_binary_extension() {
        assert_eq!(
            skip_file_reason(Path::new("assets/logo.png")),
            Some("binary or asset extension")
        );
        assert_eq!(skip_file_reason(Path::new("src/main.rs")), None);
    }
}
