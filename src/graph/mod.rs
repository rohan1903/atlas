mod build;
mod db;
mod rank;

use std::path::{Path, PathBuf};

use crate::parse::ParseOutput;
use crate::scan::InventoryFile;
use crate::store::GRAPH_DB;

pub use build::is_entrypoint;
pub use rank::{load_top_files, RankedFile};

#[derive(Debug, Clone, Copy)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
}

pub fn build_and_store(
    atlas_dir: &Path,
    inventory: &[InventoryFile],
    parse_output: &ParseOutput,
) -> Result<(PathBuf, GraphStats), String> {
    let db_path = atlas_dir.join(GRAPH_DB);
    let connection = db::open(&db_path)?;
    db::init(&connection)?;
    build::build_graph(&connection, inventory, &parse_output.files)?;
    rank::rank_files(&connection, inventory, &parse_output.files)?;
    let stats = GraphStats {
        nodes: db::count_nodes(&connection)?,
        edges: db::count_edges(&connection)?,
    };
    Ok((db_path, stats))
}

pub fn top_files(repo: &Path, limit: usize) -> Result<Vec<RankedFile>, String> {
    crate::store::require_atlas_dir(repo)?;
    let db_path = crate::store::require_graph_db(repo)?;
    let connection = db::open(&db_path)?;
    rank::load_top_files(&connection, limit)
}

pub fn open_graph(repo: &Path) -> Result<(rusqlite::Connection, std::path::PathBuf), String> {
    crate::store::require_atlas_dir(repo)?;
    let db_path = crate::store::require_graph_db(repo)?;
    let connection = db::open(&db_path)?;
    Ok((connection, db_path))
}
