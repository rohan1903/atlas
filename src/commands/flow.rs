use std::path::Path;

use crate::diagram::{self, BoxLine};
use crate::style;

pub fn run(repo: &Path, name: &str) -> Result<(), String> {
    let flow = crate::intelligence::flow::extract_flow(repo, name)?;

    println!(
        "{} {}",
        style::label("Flow:"),
        style::emphasis(&flow.query)
    );
    println!(
        "  {} {}",
        style::muted("seed"),
        style::path(&flow.seed)
    );
    println!();

    let lines: Vec<BoxLine> = flow
        .steps
        .iter()
        .map(|step| BoxLine {
            primary: step.label.clone(),
            secondary: match step.line {
                Some(line) => format!("{}:{}", step.file, line),
                None => step.file.clone(),
            },
        })
        .collect();
    diagram::print_vertical_boxes(&lines);

    println!();
    println!(
        "{}",
        style::muted("flow is approximate — macros and dynamic dispatch may be missing")
    );

    Ok(())
}
