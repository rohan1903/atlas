use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, Theme};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use termcolor::{Color as TermColor, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::parse::LanguageKind;
use crate::paths;

const DEFAULT_TEXT: (u8, u8, u8) = (212, 212, 212);
const MIN_LUMINANCE: f32 = 150.0;
const TERMINAL_BG: Color = Color {
    r: 13,
    g: 17,
    b: 23,
    a: 255,
};

struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

static HIGHLIGHTER: OnceLock<Option<Highlighter>> = OnceLock::new();
static FORCE_COLOR: AtomicBool = AtomicBool::new(false);

pub fn init_terminal_colors() {
    #[cfg(windows)]
    {
        let _ = colored::control::set_virtual_terminal(true);
    }
}

pub fn set_force_color(force: bool) {
    FORCE_COLOR.store(force, Ordering::Relaxed);
    if force {
        colored::control::set_override(true);
    }
}

pub fn should_color() -> bool {
    FORCE_COLOR.load(Ordering::Relaxed) || colored::control::SHOULD_COLORIZE.should_colorize()
}

fn color_choice() -> ColorChoice {
    if should_color() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    }
}

pub fn write_snippet_block(
    path: &str,
    start_line: usize,
    lines: &[String],
) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(color_choice());

    if !should_color() {
        for (offset, line) in lines.iter().enumerate() {
            writeln!(
                stdout,
                "    {:>4}  {}",
                start_line + offset,
                line
            )?;
        }
        return Ok(());
    }

    let Some(highlighter) = highlighter() else {
        return write_plain_snippet_block(&mut stdout, start_line, lines);
    };

    let Some(extension) = syntax_extension_for_path(path) else {
        return write_plain_snippet_block(&mut stdout, start_line, lines);
    };

    let syntax = match highlighter.syntax_set.find_syntax_by_extension(extension) {
        Some(syntax) => syntax,
        None => return write_plain_snippet_block(&mut stdout, start_line, lines),
    };

    let mut highlight_lines = HighlightLines::new(syntax, &highlighter.theme);
    let text = lines.join("\n");

    for (offset, line) in LinesWithEndings::from(text.as_str()).enumerate() {
        write!(stdout, "    {:>4}  ", start_line + offset)?;
        match highlight_lines.highlight_line(line, &highlighter.syntax_set) {
            Ok(ranges) => write_colored_ranges(&mut stdout, &ranges)?,
            Err(_) => write!(stdout, "{}", line.trim_end_matches('\n'))?,
        }
        stdout.reset()?;
        writeln!(stdout)?;
    }

    Ok(())
}

fn write_plain_snippet_block(
    stdout: &mut StandardStream,
    start_line: usize,
    lines: &[String],
) -> io::Result<()> {
    for (offset, line) in lines.iter().enumerate() {
        writeln!(stdout, "    {:>4}  {}", start_line + offset, line)?;
    }
    Ok(())
}

fn write_colored_ranges(stdout: &mut StandardStream, ranges: &[(Style, &str)]) -> io::Result<()> {
    for (style, text) in ranges {
        let (r, g, b) = terminal_fg_color(style);
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(TermColor::Rgb(r, g, b)));
        stdout.set_color(&spec)?;
        write!(stdout, "{text}")?;
    }
    Ok(())
}

fn terminal_fg_color(style: &Style) -> (u8, u8, u8) {
    let fg = style.foreground;
    if fg.a < 16 && fg.r == 0 && fg.g == 0 && fg.b == 0 {
        return DEFAULT_TEXT;
    }

    let blended = if fg.a == 255 {
        fg
    } else {
        blend_fg_color(fg, TERMINAL_BG)
    };

    ensure_readable_on_dark(blended.r, blended.g, blended.b)
}

fn ensure_readable_on_dark(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let mut rgb = (r, g, b);
    if relative_luminance(rgb.0, rgb.1, rgb.2) < 8.0 {
        return DEFAULT_TEXT;
    }

    for _ in 0..4 {
        let luminance = relative_luminance(rgb.0, rgb.1, rgb.2);
        if luminance >= MIN_LUMINANCE {
            return rgb;
        }

        let factor = MIN_LUMINANCE / luminance;
        rgb = (
            (rgb.0 as f32 * factor).min(255.0) as u8,
            (rgb.1 as f32 * factor).min(255.0) as u8,
            (rgb.2 as f32 * factor).min(255.0) as u8,
        );
    }

    DEFAULT_TEXT
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

fn syntax_extension_for_path(path: &str) -> Option<&'static str> {
    let normalized = paths::normalize_path(path);
    let file_name = normalized.rsplit('/').next()?;
    let kind = LanguageKind::from_path(Path::new(file_name))?;
    Some(match kind {
        LanguageKind::Python => "py",
        LanguageKind::TypeScript => "ts",
        LanguageKind::JavaScript => "js",
        LanguageKind::Go => "go",
        LanguageKind::C => "c",
    })
}

fn highlighter() -> Option<&'static Highlighter> {
    HIGHLIGHTER
        .get_or_init(|| {
            let syntax_set = SyntaxSet::load_defaults_newlines();
            let theme_set = syntect::highlighting::ThemeSet::load_defaults();
            let theme = theme_set
                .themes
                .get("base16-ocean.dark")
                .or_else(|| theme_set.themes.get("Solarized (dark)"))
                .or_else(|| theme_set.themes.get("base16-mocha.dark"))
                .cloned()?;

            Some(Highlighter { syntax_set, theme })
        })
        .as_ref()
}

fn blend_fg_color(fg: Color, bg: Color) -> Color {
    let alpha = if fg.a == 0 { 255 } else { fg.a as u32 };
    let r = (fg.r as u32 * alpha + bg.r as u32 * (255 - alpha)) / 255;
    let g = (fg.g as u32 * alpha + bg.g as u32 * (255 - alpha)) / 255;
    let b = (fg.b as u32 * alpha + bg.b as u32 * (255 - alpha)) / 255;

    Color {
        r: r as u8,
        g: g as u8,
        b: b as u8,
        a: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_python_from_windows_style_path() {
        assert_eq!(
            syntax_extension_for_path(r"auth\routes.py"),
            Some("py")
        );
    }

    #[test]
    fn boosts_dark_editor_colors_for_terminal() {
        let style = Style {
            foreground: Color {
                r: 36,
                g: 41,
                b: 46,
                a: 255,
            },
            ..Style::default()
        };
        let (r, g, b) = terminal_fg_color(&style);
        assert!(
            relative_luminance(r, g, b) >= MIN_LUMINANCE,
            "expected readable color, got ({r}, {g}, {b})"
        );
    }

    #[test]
    fn preserves_bright_keyword_colors() {
        let style = Style {
            foreground: Color {
                r: 198,
                g: 120,
                b: 221,
                a: 255,
            },
            ..Style::default()
        };
        assert_eq!(terminal_fg_color(&style), (198, 120, 221));
    }
}
