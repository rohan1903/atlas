use std::path::Path;

use crate::diagram::{self, BoxLine};
use crate::style;

pub fn run(repo: &Path, topic: &str) -> Result<(), String> {
    let plan = crate::intelligence::learn::build_learning_path(repo, topic)?;

    println!(
        "{} {}",
        style::label("Goal:"),
        style::emphasis(&format!("Understand {}", plan.topic))
    );
    println!(
        "  {} {}",
        style::muted("subsystem"),
        style::emphasis(&plan.subsystem)
    );
    println!();
    println!("{}", style::heading("Read order"));

    let lines: Vec<BoxLine> = plan
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| BoxLine {
            primary: format!("{}. {}", index + 1, step.path),
            secondary: step.reason.clone(),
        })
        .collect();
    diagram::print_vertical_boxes(&lines);

    println!();
    println!(
        "{} {} {}",
        style::label("Estimated time:"),
        style::score_value(plan.estimated_minutes.max(1) as i64),
        style::muted("minutes (rough: 5 min per file)")
    );

    Ok(())
}
