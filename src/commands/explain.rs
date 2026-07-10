use std::path::Path;

use crate::diagram::{self, BoxLine};
use crate::highlight;
use crate::intelligence::explain::{self, ExplainEvidence};
use crate::style;

pub fn run(repo: &Path, topic: &str) -> Result<(), String> {
    let evidence = explain::gather_evidence(repo, topic)?;

    if !explain::all_cited_paths_exist_in_graph(repo, &evidence)? {
        return Err("internal error: explain citations must come from the graph".to_string());
    }

    print_explanation(&evidence, repo)?;
    Ok(())
}

fn print_explanation(evidence: &ExplainEvidence, repo: &Path) -> Result<(), String> {
    println!(
        "{} {}",
        style::label("Repository:"),
        style::emphasis(&evidence.repository_name)
    );
    println!(
        "{} {}",
        style::label("Topic:"),
        style::emphasis(&evidence.topic)
    );
    println!(
        "  {} {}",
        style::muted("subsystem"),
        style::emphasis(&format!(
            "{} ({})",
            evidence.subsystem_name, evidence.subsystem_key
        ))
    );
    println!();

    println!("{}", style::heading("Overview"));
    let overview = explain::build_overview(evidence);
    for line in &overview.summary_lines {
        println!("  {line}");
    }
    if !overview.reading_steps.is_empty() {
        println!();
        println!("  {}", style::label("Reading order"));
        let lines: Vec<BoxLine> = overview
            .reading_steps
            .iter()
            .enumerate()
            .map(|(index, step)| BoxLine {
                primary: format!("{}. {}", index + 1, step.path),
                secondary: step.detail.clone(),
            })
            .collect();
        diagram::print_vertical_boxes(&lines);
    }
    println!();

    if !evidence.purpose.is_empty() {
        println!("{}", style::heading("Purpose"));
        for paragraph in &evidence.purpose {
            println!("  {paragraph}");
        }
        println!();
    }

    if !evidence.request_walkthrough.is_empty() {
        println!("{}", style::heading("Request walkthrough"));
        for line in &evidence.request_walkthrough {
            println!("  {line}");
        }
        println!();
    }

    if !evidence.execution_flow.is_empty() {
        println!(
            "{}",
            style::heading(explain::execution_flow_heading(evidence))
        );
        for line in &evidence.execution_flow {
            println!("  {line}");
        }
        println!();
    }

    println!("{}", style::heading("Citations"));
    if evidence.citations.is_empty() {
        println!(
            "  {}",
            style::muted("no ranked files in this subsystem — rescan the repository")
        );
    } else {
        for (index, citation) in evidence.citations.iter().enumerate() {
            let anchor = match (&citation.anchor_symbol, citation.anchor_line) {
                (Some(symbol), Some(line)) => format!(" @ {symbol}:{line}"),
                (Some(symbol), None) => format!(" @ {symbol}"),
                (None, Some(line)) => format!(" @ line {line}"),
                (None, None) => String::new(),
            };
            println!(
                "  {}. {}{} {}",
                index + 1,
                style::path(&citation.path),
                style::muted(&anchor),
                style::muted(&format!(
                    "({:.0} score, {} inbound, {})",
                    citation.score, citation.inbound_refs, citation.role
                ))
            );
        }
    }

    let snippet_count = evidence
        .citations
        .iter()
        .filter(|citation| citation.snippet.is_some())
        .count();
    if snippet_count > 0 {
        println!();
        println!("{}", style::heading("Snippets"));
        for citation in evidence
            .citations
            .iter()
            .filter(|citation| citation.snippet.is_some())
        {
            let snippet = citation.snippet.as_ref().expect("snippet checked");
            let symbol = citation.anchor_symbol.as_deref().unwrap_or("anchor");
            println!(
                "  {} {} {}",
                style::path(&citation.path),
                style::muted(&format!(
                    "({symbol}, lines {}-{}{})",
                    snippet.start_line,
                    snippet.end_line,
                    if snippet.truncated { ", truncated" } else { "" }
                )),
                style::muted(&format!("— {}", citation.role))
            );
            highlight::write_snippet_block(&citation.path, snippet.start_line, &snippet.lines)
                .map_err(|error| format!("failed to print snippet: {error}"))?;
            println!();
        }
    }

    println!("{}", style::heading("How execution gets here"));
    if evidence.entrypoints.is_empty() && evidence.wiring_hints.is_empty() {
        println!(
            "  {}",
            style::muted("no wiring hints detected — try `atlas architecture`")
        );
    } else {
        for entrypoint in &evidence.entrypoints {
            println!(
                "  - {} {}",
                style::label("entrypoint:"),
                style::path(entrypoint)
            );
        }
        for hint in &evidence.wiring_hints {
            println!("  - {}", style::muted(hint));
        }
    }

    let repo_hint = repo.display().to_string();
    println!();
    println!("{}", style::heading("Next"));
    println!(
        "  {} {}",
        style::label("learn:"),
        style::muted(&format!("atlas learn {} {}", evidence.topic, repo_hint))
    );
    println!(
        "  {} {}",
        style::label("flow:"),
        style::muted(&format!("atlas flow {} {}", evidence.topic, repo_hint))
    );

    Ok(())
}
