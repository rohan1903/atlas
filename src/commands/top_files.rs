use std::path::Path;

use crate::graph::RankedFile;
use crate::style;

const DEFAULT_LIMIT: usize = 20;

pub fn run(repo: &Path, limit: usize) -> Result<(), String> {
    let ranked = crate::graph::top_files(repo, limit)?;

    if ranked.is_empty() {
        return Err(
            "no ranked files found — run `atlas scan .` on a repository with supported source files"
                .to_string(),
        );
    }

    let repo_name = repo
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "repository".to_string());
    let repo_hint = repo.display().to_string();

    println!(
        "{} {}",
        style::label("Repository:"),
        style::emphasis(&repo_name)
    );
    println!();
    println!("{}", style::heading("Top files"));
    println!();

    for (index, file) in ranked.iter().enumerate() {
        print_ranked_line(index + 1, file);
    }

    if ranked.len() == limit {
        println!();
        println!(
            "{}",
            style::muted(&format!("showing top {limit} files — use --limit to change"))
        );
    }

    crate::commands::report::print_top_files_next(&repo_hint);

    Ok(())
}

fn print_ranked_line(rank: usize, file: &RankedFile) {
    let entrypoint = if file.is_entrypoint {
        format!(" {}", style::muted("(entrypoint)"))
    } else {
        String::new()
    };

    println!(
        "{:>2}. {:>5}  {}{}",
        rank,
        style::score_value(file.score.round() as i64),
        style::path(&file.file_path),
        entrypoint,
    );
    println!(
        "      {}",
        style::muted(&format!(
            "inbound: {}, outbound: {}, definitions: {}",
            file.inbound_refs, file.outbound_refs, file.definitions
        ))
    );
}

pub fn default_limit() -> usize {
    DEFAULT_LIMIT
}
