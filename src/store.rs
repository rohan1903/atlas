use std::path::{Path, PathBuf};

pub const ATLAS_DIR: &str = ".atlas";
pub const GRAPH_DB: &str = "graph.db";

pub fn atlas_dir(repo: &Path) -> PathBuf {
    repo.join(ATLAS_DIR)
}

pub fn graph_db_path(repo: &Path) -> PathBuf {
    atlas_dir(repo).join(GRAPH_DB)
}

pub fn require_atlas_dir(repo: &Path) -> Result<PathBuf, String> {
    let dir = atlas_dir(repo);
    if !dir.is_dir() {
        return Err(format!(
            "no Atlas data found at {} — run `atlas scan .` first",
            dir.display()
        ));
    }
    Ok(dir)
}

pub fn require_graph_db(repo: &Path) -> Result<PathBuf, String> {
    let db = graph_db_path(repo);
    if !db.is_file() {
        return Err(format!(
            "no graph database at {} — run `atlas scan .` first",
            db.display()
        ));
    }
    Ok(db)
}

pub fn resolve_repo(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("could not resolve path {}: {error}", path.display()))
}
