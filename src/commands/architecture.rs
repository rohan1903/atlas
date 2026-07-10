use std::path::Path;

use crate::intelligence::ArchitectureReport;
use crate::style;

pub fn run(repo: &Path) -> Result<(), String> {
    let report = crate::intelligence::analyze(repo)?;
    print_report(&report, repo);
    Ok(())
}

fn print_report(report: &ArchitectureReport, repo: &Path) {
    println!(
        "{} {}",
        style::label("Repository:"),
        style::emphasis(&report.repository_name)
    );
    println!();

    println!("{}", style::heading("Subsystems"));
    if report.subsystems.is_empty() {
        println!(
            "  {}",
            style::muted("none detected — try scanning a larger repository")
        );
    } else {
        for (index, subsystem) in report.subsystems.iter().enumerate() {
            println!(
                "  {}. {} {}",
                index + 1,
                style::emphasis(&subsystem.name),
                style::muted(&format!(
                    "({} files, score {:.0}, internal links {})",
                    subsystem.file_count, subsystem.total_score, subsystem.internal_links
                ))
            );
            if !subsystem.top_files.is_empty() {
                println!(
                    "     {}",
                    style::muted(&format!("top: {}", subsystem.top_files.join(", ")))
                );
            }
        }
    }

    println!();
    println!("{}", style::heading("Entrypoints"));
    if report.entrypoints.is_empty() {
        println!("  {}", style::muted("none detected"));
    } else {
        for entrypoint in &report.entrypoints {
            println!("  - {}", style::path(entrypoint));
        }
    }

    println!();
    println!("{}", style::heading("Critical files"));
    if report.critical_files.is_empty() {
        println!(
            "  {}",
            style::muted("none ranked yet — run `atlas scan` on source files")
        );
    } else {
        for (index, file) in report.critical_files.iter().enumerate() {
            println!(
                "  {}. {:>5}  {}",
                index + 1,
                style::score_value(file.score.round() as i64),
                style::path(&file.file_path)
            );
        }
    }

    let repo_hint = repo.display().to_string();
    println!();
    println!("{}", style::heading("Next"));
    println!(
        "  {} {}",
        style::label("top-files:"),
        style::muted(&format!("atlas top-files {repo_hint}"))
    );
}
