use crate::style;

const FILE_PROGRESS_INTERVAL: usize = 250;
const PARSE_PROGRESS_INTERVAL: usize = 50;

pub fn step(label: &str) {
    eprintln!("{} {}", style::info("→"), style::emphasis(label));
}

pub fn detail(message: &str) {
    eprintln!("  {}", style::muted(message));
}

pub fn file_tick(count: usize, label: &str, verbose: bool) {
    if verbose {
        return;
    }
    if count == 1 || count % FILE_PROGRESS_INTERVAL == 0 {
        eprintln!(
            "  {} {} {}...",
            style::muted(label),
            style::metric_value("files", count),
            style::muted("files")
        );
    }
}

pub fn parse_tick(count: usize, verbose: bool) {
    if verbose {
        return;
    }
    if count == 1 || count % PARSE_PROGRESS_INTERVAL == 0 {
        eprintln!(
            "  {} {} {}...",
            style::muted("parsed"),
            style::metric_value("files", count),
            style::muted("files")
        );
    }
}

pub fn done(message: &str) {
    eprintln!("{} {}", style::info("✓"), style::muted(message));
}
