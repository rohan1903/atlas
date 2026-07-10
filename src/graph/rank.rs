use std::collections::HashMap;

use rusqlite::Connection;

use crate::parse::ParsedFile;
use crate::paths;
use crate::scan::InventoryFile;

use super::build::is_entrypoint;
use super::db;

#[derive(Debug, Clone)]
pub struct RankedFile {
    pub file_path: String,
    pub score: f64,
    pub inbound_refs: usize,
    pub outbound_refs: usize,
    pub definitions: usize,
    pub is_entrypoint: bool,
}

pub fn rank_files(
    connection: &Connection,
    inventory: &[InventoryFile],
    parsed_files: &[ParsedFile],
) -> Result<Vec<RankedFile>, String> {
    let inbound = inbound_import_counts(connection)?;
    let outbound = outbound_activity_counts(connection)?;
    let definitions = definition_counts(parsed_files);

    let mut ranked = Vec::new();

    for file in inventory {
        let path = &file.path;
        let inbound_refs = *inbound.get(path).unwrap_or(&0);
        let outbound_refs = *outbound.get(path).unwrap_or(&0);
        let definition_count = *definitions.get(path).unwrap_or(&0);
        let entrypoint = is_entrypoint(path);

        let mut score =
            inbound_refs as f64 * 3.0 + outbound_refs as f64 * 0.5 + definition_count as f64 * 0.3;

        if entrypoint {
            score += 40.0;
        }

        if paths::is_test_path(path) {
            score *= 0.05;
        }

        if paths::is_project_metadata_path(path) {
            score *= 0.1;
        }

        if paths::is_deprioritized_path(path) {
            score *= 0.25;
        }

        db::upsert_file_score(
            connection,
            path,
            score,
            inbound_refs,
            outbound_refs,
            definition_count,
            entrypoint,
        )?;

        ranked.push(RankedFile {
            file_path: path.clone(),
            score,
            inbound_refs,
            outbound_refs,
            definitions: definition_count,
            is_entrypoint: entrypoint,
        });
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.file_path.cmp(&right.file_path))
    });

    Ok(ranked)
}

pub fn load_top_files(
    connection: &Connection,
    limit: usize,
    include_tests: bool,
    include_metadata: bool,
) -> Result<Vec<RankedFile>, String> {
    let fetch_limit = if include_tests && include_metadata {
        limit
    } else {
        limit.saturating_mul(12).max(limit)
    };

    let mut statement = connection
        .prepare(
            "SELECT file_path, score, inbound_refs, outbound_refs, definitions, is_entrypoint
             FROM file_scores
             ORDER BY score DESC, file_path ASC
             LIMIT ?1",
        )
        .map_err(|error| format!("could not query file scores: {error}"))?;

    let rows = statement
        .query_map([fetch_limit as i64], |row| {
            Ok(RankedFile {
                file_path: row.get(0)?,
                score: row.get(1)?,
                inbound_refs: row.get::<_, i64>(2)? as usize,
                outbound_refs: row.get::<_, i64>(3)? as usize,
                definitions: row.get::<_, i64>(4)? as usize,
                is_entrypoint: row.get::<_, i64>(5)? == 1,
            })
        })
        .map_err(|error| format!("could not read file scores: {error}"))?;

    let mut ranked = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("could not collect file scores: {error}"))?;

    if !include_tests {
        ranked.retain(|file| !paths::is_test_path(&file.file_path));
    }

    if !include_metadata {
        ranked.retain(|file| !paths::is_config_or_docs_path(&file.file_path));
    }

    ranked.truncate(limit);
    Ok(ranked)
}

fn inbound_import_counts(connection: &Connection) -> Result<HashMap<String, usize>, String> {
    let mut statement = connection
        .prepare(
            "SELECT target.file_path, COUNT(*)
             FROM edges
             JOIN nodes AS source ON edges.source_id = source.id
             JOIN nodes AS target ON edges.target_id = target.id
             WHERE edges.kind = 'IMPORTS'
               AND source.kind = 'file'
               AND target.kind = 'file'
             GROUP BY target.file_path",
        )
        .map_err(|error| format!("could not query inbound refs: {error}"))?;

    let mut counts = HashMap::new();
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })
        .map_err(|error| format!("could not read inbound refs: {error}"))?;

    for row in rows {
        let (path, count) =
            row.map_err(|error| format!("could not collect inbound refs: {error}"))?;
        counts.insert(path, count);
    }

    Ok(counts)
}

fn outbound_activity_counts(connection: &Connection) -> Result<HashMap<String, usize>, String> {
    let mut statement = connection
        .prepare(
            "SELECT source.file_path, COUNT(*)
             FROM edges
             JOIN nodes AS source ON edges.source_id = source.id
             WHERE source.kind = 'file'
               AND edges.kind IN ('IMPORTS', 'CALLS')
             GROUP BY source.file_path",
        )
        .map_err(|error| format!("could not query outbound refs: {error}"))?;

    let mut counts = HashMap::new();
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })
        .map_err(|error| format!("could not read outbound refs: {error}"))?;

    for row in rows {
        let (path, count) =
            row.map_err(|error| format!("could not collect outbound refs: {error}"))?;
        counts.insert(path, count);
    }

    Ok(counts)
}

fn definition_counts(parsed_files: &[ParsedFile]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for parsed in parsed_files {
        counts.insert(parsed.path.clone(), parsed.definitions.len());
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::build::build_graph;
    use crate::graph::db::init;
    use crate::parse::{Definition, Import, ParsedFile};

    #[test]
    fn ranks_imported_file_higher() {
        let connection = Connection::open_in_memory().expect("open db");
        init(&connection).expect("init db");
        let inventory = vec![
            InventoryFile {
                path: "core.c".to_string(),
                size_bytes: 1,
            },
            InventoryFile {
                path: "include/util.h".to_string(),
                size_bytes: 1,
            },
            InventoryFile {
                path: "notes.txt".to_string(),
                size_bytes: 1,
            },
        ];
        let parsed = vec![
            ParsedFile {
                path: "core.c".to_string(),
                language: "c".to_string(),
                definitions: vec![Definition {
                    kind: "function".to_string(),
                    name: "run".to_string(),
                    line: 1,
                }],
                imports: vec![Import {
                    kind: "include".to_string(),
                    target: "util.h".to_string(),
                    line: 1,
                }],
                calls: vec![],
            },
            ParsedFile {
                path: "include/util.h".to_string(),
                language: "c".to_string(),
                definitions: vec![Definition {
                    kind: "function".to_string(),
                    name: "helper".to_string(),
                    line: 2,
                }],
                imports: vec![],
                calls: vec![],
            },
        ];

        build_graph(&connection, &inventory, &parsed).expect("build graph");
        let ranked = rank_files(&connection, &inventory, &parsed).expect("rank files");

        let util_rank = ranked
            .iter()
            .position(|file| file.file_path == "include/util.h")
            .expect("util.h ranked");
        let notes_rank = ranked
            .iter()
            .position(|file| file.file_path == "notes.txt")
            .expect("notes ranked");

        assert!(util_rank < notes_rank);
        assert!(ranked[util_rank].inbound_refs >= 1);
    }

    #[test]
    fn excludes_test_files_by_default() {
        use crate::parse::Call;

        let connection = Connection::open_in_memory().expect("open db");
        init(&connection).expect("init db");
        let inventory = vec![
            InventoryFile {
                path: "app.py".to_string(),
                size_bytes: 1,
            },
            InventoryFile {
                path: "tests/test_app.py".to_string(),
                size_bytes: 1,
            },
        ];
        let parsed = vec![
            ParsedFile {
                path: "app.py".to_string(),
                language: "python".to_string(),
                definitions: vec![Definition {
                    kind: "function".to_string(),
                    name: "run".to_string(),
                    line: 1,
                }],
                imports: vec![],
                calls: vec![],
            },
            ParsedFile {
                path: "tests/test_app.py".to_string(),
                language: "python".to_string(),
                definitions: vec![Definition {
                    kind: "function".to_string(),
                    name: "test_run".to_string(),
                    line: 1,
                }],
                imports: vec![],
                calls: vec![Call {
                    target: "mock".to_string(),
                    line: 2,
                }],
            },
        ];

        build_graph(&connection, &inventory, &parsed).expect("build graph");
        rank_files(&connection, &inventory, &parsed).expect("rank files");

        let production =
            load_top_files(&connection, 10, false, false).expect("production top files");
        assert!(production
            .iter()
            .all(|file| !paths::is_test_path(&file.file_path)));
        assert!(production
            .iter()
            .all(|file| !paths::is_config_or_docs_path(&file.file_path)));
        assert_eq!(production[0].file_path, "app.py");

        let with_tests = load_top_files(&connection, 10, true, false).expect("with tests");
        assert!(with_tests
            .iter()
            .any(|file| file.file_path == "tests/test_app.py"));
    }
}
