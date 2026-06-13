mod extract;
mod language;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use language::LanguageKind;

const SYMBOLS_FILE: &str = "symbols.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    pub kind: String,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub kind: String,
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub path: String,
    pub language: String,
    pub definitions: Vec<Definition>,
    pub imports: Vec<Import>,
    pub calls: Vec<Call>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ParseSummary {
    pub parsed: usize,
    pub unsupported: usize,
    pub failed: usize,
    pub too_large: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParseOutput {
    pub version: u32,
    pub summary: ParseSummary,
    pub files: Vec<ParsedFile>,
}

pub fn parse_inventory(
    root: &Path,
    inventory_paths: &[String],
    verbose: bool,
) -> Result<ParseOutput, String> {
    let mut output = ParseOutput {
        version: 1,
        summary: ParseSummary::default(),
        files: Vec::new(),
    };

    for relative_path in inventory_paths {
        let absolute_path = root.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));

        let language = match LanguageKind::from_path(&absolute_path) {
            Some(language) => language,
            None => {
                output.summary.unsupported += 1;
                continue;
            }
        };

        let source = match fs::read_to_string(&absolute_path) {
            Ok(source) => source,
            Err(error) => {
                output.summary.failed += 1;
                if verbose {
                    eprintln!(
                        "{}",
                        crate::style::error_verbose(
                            &format!("parse read failed: {relative_path}"),
                            &error.to_string(),
                        )
                    );
                }
                continue;
            }
        };

        if source.len() > 5 * 1024 * 1024 {
            output.summary.too_large += 1;
            if verbose {
                eprintln!(
                    "{}",
                    crate::style::skip_verbose(
                        "parse skip:",
                        relative_path,
                        "file larger than 5MB",
                    )
                );
            }
            continue;
        }

        match extract::parse_file(relative_path, language, &source) {
            Some(parsed) => {
                output.summary.parsed += 1;
                if verbose {
                    eprintln!(
                        "{} {} {}",
                        crate::style::info("parsed:"),
                        crate::style::path(relative_path),
                        crate::style::muted(&format!(
                            "({} defs, {} imports, {} calls)",
                            parsed.definitions.len(),
                            parsed.imports.len(),
                            parsed.calls.len()
                        ))
                    );
                } else {
                    crate::progress::parse_tick(output.summary.parsed, verbose);
                }
                output.files.push(parsed);
            }
            None => {
                output.summary.failed += 1;
                if verbose {
                    eprintln!(
                        "{}",
                        crate::style::error_verbose(
                            &format!("parse failed: {relative_path}"),
                            "tree-sitter could not parse file",
                        )
                    );
                }
            }
        }
    }

    output.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(output)
}

pub fn write_symbols(atlas_dir: &Path, output: &ParseOutput) -> Result<std::path::PathBuf, String> {
    let symbols_path = atlas_dir.join(SYMBOLS_FILE);
    let json = serde_json::to_string_pretty(output)
        .map_err(|error| format!("could not serialize symbols: {error}"))?;
    fs::write(&symbols_path, json)
        .map_err(|error| format!("could not write {}: {error}", symbols_path.display()))?;
    Ok(symbols_path)
}

pub fn symbol_totals(output: &ParseOutput) -> (usize, usize, usize) {
    let mut definitions = 0;
    let mut imports = 0;
    let mut calls = 0;
    for file in &output.files {
        definitions += file.definitions.len();
        imports += file.imports.len();
        calls += file.calls.len();
    }
    (definitions, imports, calls)
}

pub fn supported_languages_label() -> &'static str {
    "Python, TypeScript, JavaScript, Go, C"
}

pub fn load_symbols(atlas_dir: &Path) -> Result<ParseOutput, String> {
    let symbols_path = atlas_dir.join(SYMBOLS_FILE);
    let json = fs::read_to_string(&symbols_path)
        .map_err(|error| format!("could not read {}: {error}", symbols_path.display()))?;
    serde_json::from_str(&json).map_err(|error| format!("could not parse symbols.json: {error}"))
}
