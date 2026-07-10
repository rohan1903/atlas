use crate::graph::GraphStats;
use crate::parse::ParseSummary;
use crate::style;

pub fn print_next_steps(repo_hint: &str) {
    println!();
    println!("{}", style::heading("Next"));
    println!(
        "  {} {}",
        style::label("architecture:"),
        style::muted(&format!("atlas architecture {repo_hint}"))
    );
    println!(
        "  {} {}",
        style::label("top-files:"),
        style::muted(&format!("atlas top-files {repo_hint}"))
    );
    println!(
        "  {} {}",
        style::label("flow:"),
        style::muted(&format!("atlas flow <name> {repo_hint}"))
    );
    println!(
        "  {} {}",
        style::label("learn:"),
        style::muted(&format!("atlas learn <topic> {repo_hint}"))
    );
}
pub fn print_top_files_next(repo_hint: &str) {
    println!();
    println!("{}", style::heading("Next"));
    println!(
        "  {} {}",
        style::label("architecture:"),
        style::muted(&format!("atlas architecture {repo_hint}"))
    );
}

pub fn print_scan_complete(
    inventory_files: usize,
    parse_summary: &ParseSummary,
    symbol_totals: (usize, usize, usize),
    graph_stats: GraphStats,
    ranked_files: usize,
) {
    let (definitions, imports, calls) = symbol_totals;

    println!();
    println!("{}", style::heading("Complete"));
    println!(
        "  {} {}",
        style::label("inventory:"),
        style::metric_value("files", inventory_files)
    );
    println!(
        "  {} {} parsed, {} unsupported, {} failed",
        style::label("symbols:"),
        style::metric_value("files", parse_summary.parsed),
        style::metric_value("skipped", parse_summary.unsupported),
        style::metric_value("errors", parse_summary.failed)
    );
    println!(
        "      {}",
        style::muted(&format!(
            "{definitions} definitions, {imports} imports, {calls} calls"
        ))
    );
    println!(
        "  {} {} nodes, {} edges",
        style::label("graph:"),
        style::metric_value("files", graph_stats.nodes),
        style::metric_value("files", graph_stats.edges)
    );
    println!(
        "  {} {}",
        style::label("ranked:"),
        style::metric_value("files", ranked_files)
    );
}
