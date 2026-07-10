use colored::Colorize;

pub fn heading(text: &str) -> String {
    text.bold().cyan().to_string()
}

pub fn label(text: &str) -> String {
    text.bold().to_string()
}

pub fn path(text: &str) -> String {
    text.blue().to_string()
}

pub fn emphasis(text: &str) -> String {
    text.bold().white().to_string()
}

pub fn muted(text: &str) -> String {
    text.dimmed().to_string()
}

pub fn warning(text: &str) -> String {
    text.yellow().bold().to_string()
}

pub fn error(text: &str) -> String {
    text.red().bold().to_string()
}

pub fn info(text: &str) -> String {
    text.cyan().to_string()
}

pub fn score_value(value: i64) -> String {
    value.to_string().green().bold().to_string()
}

pub fn metric_value(name: &str, value: usize) -> String {
    match name {
        "files" => value.to_string().green().bold().to_string(),
        "skipped" if value > 0 => value.to_string().yellow().bold().to_string(),
        "skipped" => value.to_string().dimmed().to_string(),
        "errors" if value > 0 => value.to_string().red().bold().to_string(),
        "errors" => value.to_string().green().to_string(),
        _ => value.to_string(),
    }
}

pub fn skip_verbose(kind: &str, item_path: &str, reason: &str) -> String {
    format!(
        "{} {} {}",
        warning(kind),
        path(item_path),
        muted(&format!("({reason})"))
    )
}

pub fn error_verbose(context: &str, detail: &str) -> String {
    format!(
        "{} {}",
        error("error:"),
        format!("{context} ({detail})").red()
    )
}
