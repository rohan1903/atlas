use std::path::Path;

use crate::diagram::{self, BoxLine};
use crate::style;

pub fn run(repo: &Path, name: &str, verbose: bool) -> Result<(), String> {
    let flow = crate::intelligence::flow::extract_flow(repo, name)?;
    let display_steps = crate::intelligence::flow::compress_flow_steps(&flow.steps, verbose);

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
    if !verbose && display_steps.len() < flow.steps.len() {
        println!(
            "  {} {}",
            style::muted("showing"),
            style::muted(&format!(
                "primary path ({} of {} steps — use --verbose for full trace)",
                display_steps.len(),
                flow.steps.len()
            ))
        );
    }
    println!();

    let lines: Vec<BoxLine> = display_steps
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
