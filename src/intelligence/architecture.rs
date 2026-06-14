use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::graph::{is_entrypoint, load_top_files, open_graph, RankedFile};
use crate::paths;

#[derive(Debug, Clone)]
pub struct Subsystem {
    #[allow(dead_code)]
    pub key: String,
    pub name: String,
    pub file_count: usize,
    pub total_score: f64,
    pub internal_links: usize,
    pub top_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArchitectureReport {
    pub repository_name: String,
    pub subsystems: Vec<Subsystem>,
    pub entrypoints: Vec<String>,
    pub critical_files: Vec<RankedFile>,
}

const MAX_SUBSYSTEMS: usize = 8;
const CRITICAL_FILE_COUNT: usize = 5;
const ENTRYPOINT_LIMIT: usize = 10;

pub fn analyze(repo: &Path) -> Result<ArchitectureReport, String> {
    let (connection, _db_path) = open_graph(repo)?;
    let ranked = load_top_files(&connection, usize::MAX, false, false)?;
    let import_edges = load_import_edges(&connection)?;

    let repository_name = repo
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "repository".to_string());

    let code_files: Vec<RankedFile> = ranked
        .iter()
        .filter(|file| !paths::is_excluded_from_clustering(&file.file_path))
        .cloned()
        .collect();

    let subsystems = detect_subsystems(&code_files, &import_edges);
    let entrypoints = detect_entrypoints(&code_files);
    let critical_files: Vec<RankedFile> = code_files
        .iter()
        .take(CRITICAL_FILE_COUNT)
        .cloned()
        .collect();
    Ok(ArchitectureReport {
        repository_name,
        subsystems,
        entrypoints,
        critical_files,
    })
}

fn detect_subsystems(ranked: &[RankedFile], import_edges: &[(String, String)]) -> Vec<Subsystem> {
    let mut clusters: HashMap<String, SubsystemAccumulator> = HashMap::new();

    for file in ranked {
        if paths::is_excluded_from_clustering(&file.file_path) {
            continue;
        }

        let key = subsystem_key(&file.file_path);
        let entry = clusters.entry(key.clone()).or_insert_with(|| SubsystemAccumulator {
            key: key.clone(),
            name: display_subsystem_name(&key),
            file_count: 0,
            total_score: 0.0,
            files: Vec::new(),
        });
        entry.file_count += 1;
        entry.total_score += file.score;
        entry.files.push((file.file_path.clone(), file.score));
    }

    let file_to_subsystem: HashMap<String, String> = ranked
        .iter()
        .filter(|file| !paths::is_excluded_from_clustering(&file.file_path))
        .map(|file| (file.file_path.clone(), subsystem_key(&file.file_path)))
        .collect();

    let mut internal_links: HashMap<String, usize> = HashMap::new();
    for (source, target) in import_edges {
        let Some(source_key) = file_to_subsystem.get(source) else {
            continue;
        };
        let Some(target_key) = file_to_subsystem.get(target) else {
            continue;
        };
        if source_key == target_key {
            *internal_links.entry(source_key.clone()).or_insert(0) += 1;
        }
    }

    let mut subsystems: Vec<Subsystem> = clusters
        .into_values()
        .map(|cluster| {
            let mut files = cluster.files;
            files.sort_by(|left, right| {
                right
                    .1
                    .partial_cmp(&left.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Subsystem {
                key: cluster.key.clone(),
                name: cluster.name,
                file_count: cluster.file_count,
                total_score: cluster.total_score,
                internal_links: *internal_links.get(&cluster.key).unwrap_or(&0),
                top_files: files
                    .into_iter()
                    .take(3)
                    .map(|(path, _)| path)
                    .collect(),
            }
        })
        .filter(|subsystem| subsystem.file_count > 0)
        .collect();

    subsystems.sort_by(|left, right| {
        right
            .total_score
            .partial_cmp(&left.total_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.file_count.cmp(&left.file_count))
            .then_with(|| left.name.cmp(&right.name))
    });

    subsystems.truncate(MAX_SUBSYSTEMS);
    subsystems
}

fn detect_entrypoints(ranked: &[RankedFile]) -> Vec<String> {
    let mut entrypoints: Vec<String> = ranked
        .iter()
        .filter(|file| {
            !paths::is_excluded_from_clustering(&file.file_path)
                && (file.is_entrypoint
                    || is_entrypoint(&file.file_path)
                    || (is_repo_root_file(&file.file_path)
                        && file.outbound_refs >= 2
                        && file.inbound_refs == 0))
        })
        .map(|file| file.file_path.clone())
        .collect();

    entrypoints.sort_by(|left, right| {
        let left_score = ranked
            .iter()
            .find(|file| &file.file_path == left)
            .map(|file| file.score)
            .unwrap_or(0.0);
        let right_score = ranked
            .iter()
            .find(|file| &file.file_path == right)
            .map(|file| file.score)
            .unwrap_or(0.0);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.cmp(right))
    });
    entrypoints.dedup();
    entrypoints.truncate(ENTRYPOINT_LIMIT);
    entrypoints
}

fn load_import_edges(connection: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT source.file_path, target.file_path
             FROM edges
             JOIN nodes AS source ON edges.source_id = source.id
             JOIN nodes AS target ON edges.target_id = target.id
             WHERE edges.kind = 'IMPORTS'
               AND source.kind = 'file'
               AND target.kind = 'file'",
        )
        .map_err(|error| format!("could not query import edges: {error}"))?;

    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|error| format!("could not read import edges: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not collect import edges: {error}"))
}

pub fn subsystem_key(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|part| !part.is_empty()).collect();

    match parts.len() {
        0 => "(root)".to_string(),
        1 => "(root)".to_string(),
        2 => parts[0].to_string(),
        3 => format!("{}/{}", parts[0], parts[1]),
        _ => format!("{}/{}/{}", parts[0], parts[1], parts[2]),
    }
}

fn display_subsystem_name(key: &str) -> String {
    if key == "(root)" {
        return "Root".to_string();
    }

    key.split('/')
        .map(humanize_segment)
        .collect::<Vec<_>>()
        .join(" / ")
}

fn humanize_segment(segment: &str) -> String {
    segment
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_repo_root_file(path: &str) -> bool {
    path.replace('\\', "/").matches('/').count() == 0
}

struct SubsystemAccumulator {
    key: String,
    name: String,
    file_count: usize,
    total_score: f64,
    files: Vec<(String, f64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsystem_key_groups_kernel_style_paths() {
        assert_eq!(
            subsystem_key("drivers/gpu/drm/drm_drv.c"),
            "drivers/gpu/drm"
        );
        assert_eq!(subsystem_key("core.c"), "(root)");
        assert_eq!(subsystem_key("auth/service.py"), "auth");
    }

    #[test]
    fn humanizes_directory_names() {
        assert_eq!(
            display_subsystem_name("drivers/gpu/drm"),
            "Drivers / Gpu / Drm"
        );
    }

    #[test]
    fn documentation_files_are_detected() {
        assert!(paths::is_documentation_file("README.md"));
        assert!(paths::is_documentation_file("docs/guide.rst"));
        assert!(!paths::is_documentation_file("main.py"));
    }
}