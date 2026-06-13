use rusqlite::{params, Connection};

pub fn init(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS nodes (
                id INTEGER PRIMARY KEY,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                file_path TEXT,
                line INTEGER
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_unique
                ON nodes(kind, name, COALESCE(file_path, ''), COALESCE(line, -1));

            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY,
                source_id INTEGER NOT NULL,
                target_id INTEGER NOT NULL,
                kind TEXT NOT NULL,
                FOREIGN KEY(source_id) REFERENCES nodes(id),
                FOREIGN KEY(target_id) REFERENCES nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);
            CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);

            CREATE TABLE IF NOT EXISTS file_scores (
                file_path TEXT PRIMARY KEY,
                score REAL NOT NULL,
                inbound_refs INTEGER NOT NULL,
                outbound_refs INTEGER NOT NULL,
                definitions INTEGER NOT NULL,
                is_entrypoint INTEGER NOT NULL
            );
            ",
        )
        .map_err(|error| format!("could not initialize graph schema: {error}"))?;

    Ok(())
}

pub fn clear(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            DELETE FROM edges;
            DELETE FROM nodes;
            DELETE FROM file_scores;
            ",
        )
        .map_err(|error| format!("could not clear graph tables: {error}"))?;
    Ok(())
}

pub fn open(db_path: &std::path::Path) -> Result<Connection, String> {
    Connection::open(db_path).map_err(|error| format!("could not open graph db: {error}"))
}

pub fn upsert_node(
    connection: &Connection,
    kind: &str,
    name: &str,
    file_path: Option<&str>,
    line: Option<usize>,
) -> Result<i64, String> {
    connection
        .execute(
            "INSERT OR IGNORE INTO nodes (kind, name, file_path, line) VALUES (?1, ?2, ?3, ?4)",
            params![kind, name, file_path, line.map(|value| value as i64)],
        )
        .map_err(|error| format!("could not insert node: {error}"))?;

    connection
        .query_row(
            "SELECT id FROM nodes WHERE kind = ?1 AND name = ?2
             AND COALESCE(file_path, '') = COALESCE(?3, '')
             AND COALESCE(line, -1) = COALESCE(?4, -1)",
            params![kind, name, file_path, line.map(|value| value as i64)],
            |row| row.get(0),
        )
        .map_err(|error| format!("could not fetch node id: {error}"))
}

pub fn insert_edge(
    connection: &Connection,
    source_id: i64,
    target_id: i64,
    kind: &str,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO edges (source_id, target_id, kind) VALUES (?1, ?2, ?3)",
            params![source_id, target_id, kind],
        )
        .map_err(|error| format!("could not insert edge: {error}"))?;
    Ok(())
}

pub fn upsert_file_score(
    connection: &Connection,
    file_path: &str,
    score: f64,
    inbound_refs: usize,
    outbound_refs: usize,
    definitions: usize,
    is_entrypoint: bool,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO file_scores (file_path, score, inbound_refs, outbound_refs, definitions, is_entrypoint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(file_path) DO UPDATE SET
               score = excluded.score,
               inbound_refs = excluded.inbound_refs,
               outbound_refs = excluded.outbound_refs,
               definitions = excluded.definitions,
               is_entrypoint = excluded.is_entrypoint",
            params![
                file_path,
                score,
                inbound_refs as i64,
                outbound_refs as i64,
                definitions as i64,
                is_entrypoint as i64,
            ],
        )
        .map_err(|error| format!("could not write file score: {error}"))?;
    Ok(())
}

pub fn count_nodes(connection: &Connection) -> Result<usize, String> {
    connection
        .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))
        .map_err(|error| format!("could not count nodes: {error}"))
}

pub fn count_edges(connection: &Connection) -> Result<usize, String> {
    connection
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .map_err(|error| format!("could not count edges: {error}"))
}
