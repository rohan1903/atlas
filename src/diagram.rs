use crate::style;

const MIN_INNER_WIDTH: usize = 20;
const TERMINAL_WIDTH: usize = 80;

#[derive(Debug, Clone)]
pub struct BoxLine {
    pub primary: String,
    pub secondary: String,
}

pub fn print_vertical_boxes(lines: &[BoxLine]) {
    if lines.is_empty() {
        return;
    }

    let inner_width = inner_width_for_lines(lines);
    let indent = diagram_indent(inner_width + 2);
    let tee_col = indent.len() + 1 + inner_width / 2;

    for (index, line) in lines.iter().enumerate() {
        let is_last = index == lines.len() - 1;

        print_border_top(&indent, inner_width);
        print_box_row(&indent, inner_width, &line.primary, style::emphasis);
        print_box_row(&indent, inner_width, &line.secondary, style::muted);

        if is_last {
            print_border_bottom_closed(&indent, inner_width);
        } else {
            print_border_bottom_tee(&indent, inner_width);
            print_connector(tee_col);
        }
    }
}

fn print_border_top(indent: &str, inner_width: usize) {
    let border = format!("╭{}╮", "─".repeat(inner_width));
    println!("{}{}", indent, style::muted(&border));
}

fn print_box_row(indent: &str, inner_width: usize, text: &str, styler: fn(&str) -> String) {
    let len = char_count(text);
    let left = (inner_width.saturating_sub(len)) / 2;
    let right = inner_width.saturating_sub(len).saturating_sub(left);

    println!(
        "{}{}{}{}{}{}",
        indent,
        style::muted("│"),
        " ".repeat(left),
        styler(text),
        " ".repeat(right),
        style::muted("│")
    );
}

fn print_border_bottom_closed(indent: &str, inner_width: usize) {
    let border = format!("╰{}╯", "─".repeat(inner_width));
    println!("{}{}", indent, style::muted(&border));
}

fn print_border_bottom_tee(indent: &str, inner_width: usize) {
    let left = inner_width / 2;
    let right = inner_width.saturating_sub(left).saturating_sub(1);
    let border = format!("╰{}{}{}╯", "─".repeat(left), "┬", "─".repeat(right));
    println!("{}{}", indent, style::muted(&border));
}

fn print_connector(tee_col: usize) {
    println!("{}│", " ".repeat(tee_col));
    println!("{}▼", " ".repeat(tee_col));
}

fn inner_width_for_lines(lines: &[BoxLine]) -> usize {
    let mut widest = MIN_INNER_WIDTH;

    for line in lines {
        widest = widest.max(char_count(&line.primary));
        widest = widest.max(char_count(&line.secondary));
    }

    widest + 4
}

fn diagram_indent(box_outer_width: usize) -> String {
    let margin = TERMINAL_WIDTH.saturating_sub(box_outer_width).max(4) / 2;
    " ".repeat(margin)
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_width_fits_longest_line_pair() {
        let lines = vec![BoxLine {
            primary: "1. auth/routes.py".to_string(),
            secondary: "HTTP routes and handlers".to_string(),
        }];
        assert!(inner_width_for_lines(&lines) >= char_count("HTTP routes and handlers") + 4);
    }
}
