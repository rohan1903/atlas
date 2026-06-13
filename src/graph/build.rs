use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::parse::ParsedFile;
use crate::scan::InventoryFile;

use super::db;

const ENTRYPOINT_FILENAMES: &[&str] = &[
    "main.py", "app.py", "wsgi.py", "manage.py", "main.go", "index.ts", "index.js", "main.ts",
    "server.ts", "app.ts", "main.c", "init.c", "mod.rs", "lib.rs", "main.rs",
];

pub struct FileIndex {
    exact: HashMap<String, String>,
    suffixes: HashMap<String, Vec<String>>,
}

impl FileIndex {
    pub fn from_inventory(files: &[InventoryFile]) -> Self {
        let mut exact = HashMap::new();
        let mut suffixes: HashMap<String, Vec<String>> = HashMap::new();

        for file in files {
            exact.insert(file.path.clone(), file.path.clone());
            let normalized = file.path.replace('\\', "/");
            suffixes
                .entry(normalized.clone())
                .or_default()
                .push(file.path.clone());

            if let Some(suffix) = normalized.rsplit('/').next() {
                suffixes
                    .entry(suffix.to_string())
                    .or_default()
                    .push(file.path.clone());
            }
        }

        Self { exact, suffixes }
    }

    pub fn resolve_import(&self, source_file: &str, target: &str) -> Option<String> {
        let target = target.trim().trim_matches(&['"', '\''][..]);
        if target.is_empty() {
            return None;
        }

        if let Some(path) = self.exact.get(target) {
            return Some(path.clone());
        }

        let normalized = target.replace('\\', "/");

        if normalized.starts_with("./") || normalized.starts_with("../") {
            if let Some(resolved) = resolve_relative(source_file, &normalized) {
                return self.try_candidates(&resolved);
            }
        }

        if normalized.ends_with(".py") || normalized.ends_with(".go") {
            return self.try_candidates(&normalized);
        }

        if let Some(path) = self.find_suffix(&normalized) {
            return Some(path);
        }

        let python_path = normalized.replace('.', "/");
        if let Some(path) = self.try_candidates(&format!("{python_path}.py")) {
            return Some(path);
        }
        if let Some(path) = self.try_candidates(&format!("{python_path}/__init__.py")) {
            return Some(path);
        }

        let js_variants = [
            format!("{normalized}.ts"),
            format!("{normalized}.tsx"),
            format!("{normalized}.js"),
            format!("{normalized}.jsx"),
            format!("{normalized}/index.ts"),
            format!("{normalized}/index.js"),
        ];
        for candidate in js_variants {
            if let Some(path) = self.try_candidates(&candidate) {
                return Some(path);
            }
        }

        None
    }

    fn try_candidates(&self, candidate: &str) -> Option<String> {
        if let Some(path) = self.exact.get(candidate) {
            return Some(path.clone());
        }
        self.find_suffix(candidate)
    }

    fn find_suffix(&self, suffix: &str) -> Option<String> {
        let normalized = suffix.replace('\\', "/");
        if let Some(matches) = self.suffixes.get(&normalized) {
            return matches.first().cloned();
        }

        let mut best: Option<String> = None;
        for file in self.exact.values() {
            let file_norm = file.replace('\\', "/");
            if file_norm.ends_with(&normalized) {
                match &best {
                    None => best = Some(file.clone()),
                    Some(current) if file_norm.len() < current.replace('\\', "/").len() => {
                        best = Some(file.clone());
                    }
                    _ => {}
                }
            }
        }
        best
    }
}

fn resolve_relative(source_file: &str, target: &str) -> Option<String> {
    let source = Path::new(source_file);
    let parent = source.parent()?;
    let joined = parent.join(target.replace('/', std::path::MAIN_SEPARATOR_STR));
    let normalized = joined.to_string_lossy().replace('\\', "/");
    Some(normalized)
}

pub fn is_entrypoint(file_path: &str) -> bool {
    let normalized = file_path.replace('\\', "/");
    if let Some(name) = normalized.rsplit('/').next() {
        if ENTRYPOINT_FILENAMES.contains(&name) {
            let depth = normalized.matches('/').count();
            return depth <= 1;
        }
        if matches!(name, "main" | "index" | "mod" | "lib") {
            return true;
        }
    }
    false
}

pub fn build_graph(
    connection: &Connection,
    inventory: &[InventoryFile],
    parsed_files: &[ParsedFile],
) -> Result<(), String> {
    db::clear(connection)?;

    let index = FileIndex::from_inventory(inventory);
    let mut file_node_ids: HashMap<String, i64> = HashMap::new();
    let mut symbol_node_ids: HashMap<(String, String, usize), i64> = HashMap::new();
    let mut import_targets: Vec<(String, String)> = Vec::new();

    for file in inventory {
        let node_id = db::upsert_node(connection, "file", &file.path, Some(&file.path), None)?;
        file_node_ids.insert(file.path.clone(), node_id);
    }

    for parsed in parsed_files {
        let file_id = *file_node_ids
            .get(&parsed.path)
            .ok_or_else(|| format!("missing file node for {}", parsed.path))?;

        for definition in &parsed.definitions {
            let symbol_id = db::upsert_node(
                connection,
                map_definition_kind(&definition.kind),
                &definition.name,
                Some(&parsed.path),
                Some(definition.line),
            )?;
            symbol_node_ids.insert(
                (parsed.path.clone(), definition.name.clone(), definition.line),
                symbol_id,
            );
            db::insert_edge(connection, file_id, symbol_id, "DEFINES")?;
        }

        for import in &parsed.imports {
            let module_name = format!("{}:{}", import.kind, import.target);
            let module_id =
                db::upsert_node(connection, "module", &module_name, None, Some(import.line))?;
            db::insert_edge(connection, file_id, module_id, "IMPORTS")?;

            if let Some(resolved) = index.resolve_import(&parsed.path, &import.target) {
                import_targets.push((resolved, parsed.path.clone()));
            }
        }

        for call in &parsed.calls {
            let callee_id = db::upsert_node(
                connection,
                "call",
                &call.target,
                Some(&parsed.path),
                Some(call.line),
            )?;
            db::insert_edge(connection, file_id, callee_id, "CALLS")?;
        }
    }

    for (target_file, source_file) in import_targets {
        if let (Some(&target_id), Some(&source_id)) =
            (file_node_ids.get(&target_file), file_node_ids.get(&source_file))
        {
            if target_id != source_id {
                db::insert_edge(connection, source_id, target_id, "IMPORTS")?;
            }
        }
    }

    Ok(())
}

fn map_definition_kind(kind: &str) -> &str {
    match kind {
        "class" => "class",
        "struct" => "class",
        "type" => "class",
        _ => "function",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_c_include_suffix() {
        let files = vec![
            InventoryFile {
                path: "include/linux/sched.h".to_string(),
                size_bytes: 1,
            },
            InventoryFile {
                path: "kernel/sched/core.c".to_string(),
                size_bytes: 1,
            },
        ];
        let index = FileIndex::from_inventory(&files);
        let resolved = index.resolve_import("kernel/sched/core.c", "linux/sched.h");
        assert_eq!(resolved.as_deref(), Some("include/linux/sched.h"));
    }

    #[test]
    fn detects_entrypoint_main_c() {
        assert!(is_entrypoint("init/main.c"));
    }

    #[test]
    fn nested_wsgi_is_not_entrypoint() {
        assert!(!is_entrypoint("starlette/middleware/wsgi.py"));
        assert!(is_entrypoint("wsgi.py"));
    }
}
