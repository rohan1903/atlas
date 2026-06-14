use std::fs;
use std::path::Path;

use crate::parse::{Definition, ParsedFile};

pub const MAX_SNIPPET_LINES: usize = 32;

const ENTRY_METHODS: &[&str] = &[
    "__call__",
    "dispatch",
    "handle",
    "process_request",
    "login",
    "register",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnippet {
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<String>,
    pub truncated: bool,
}

pub fn read_definition_snippet(
    repo: &Path,
    rel_path: &str,
    parsed: &ParsedFile,
    topic: &str,
    role: &str,
    preferred_symbol: Option<&str>,
) -> Result<FileSnippet, String> {
    let (anchor_line, _symbol) = resolve_anchor(parsed, topic, role, preferred_symbol)
        .ok_or_else(|| format!("no anchor symbol found in {rel_path}"))?;

    let absolute = repo.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    let content = fs::read_to_string(&absolute)
        .map_err(|error| format!("could not read {}: {error}", absolute.display()))?;

    let file_lines: Vec<&str> = content.lines().collect();
    if file_lines.is_empty() {
        return Err(format!("file is empty: {rel_path}"));
    }

    extract_definition_block(&file_lines, anchor_line)
}

pub fn resolve_anchor(
    parsed: &ParsedFile,
    topic: &str,
    role: &str,
    preferred_symbol: Option<&str>,
) -> Option<(usize, String)> {
    if let Some(symbol) = preferred_symbol {
        if let Some(definition) = find_definition_by_name(parsed, symbol) {
            if definition.kind == "class" {
                if let Some(method) = pick_method_after_line(parsed, definition.line) {
                    return Some(method);
                }
            }
            return Some((definition.line, definition.name.clone()));
        }
    }

    if let Some(anchor) = pick_best_topic_anchor(parsed, topic, role) {
        return Some(anchor);
    }

    if let Some((line, name)) = pick_entry_method(parsed) {
        return Some((line, name));
    }

    let role_lower = role.to_lowercase();
    if role_lower.contains("http") || role_lower.contains("route") || role_lower.contains("handler")
    {
        if let Some(definition) = parsed.definitions.iter().find(|definition| {
            definition.kind == "function"
                && (definition.name.to_lowercase().contains("handler")
                    || definition.name.to_lowercase().contains("route"))
        }) {
            return Some((definition.line, definition.name.clone()));
        }
    }

    if role_lower.contains("service") || role_lower.contains("business") {
        if let Some(definition) = first_non_init_function(parsed) {
            return Some((definition.line, definition.name.clone()));
        }
    }

    parsed
        .definitions
        .iter()
        .find(|definition| definition.kind == "class")
        .or_else(|| parsed.definitions.first())
        .map(|definition| (definition.line, definition.name.clone()))
}

fn pick_best_topic_anchor(
    parsed: &ParsedFile,
    topic: &str,
    role: &str,
) -> Option<(usize, String)> {
    let topic_lower = topic.trim().to_lowercase();
    if topic_lower.is_empty() {
        return None;
    }

    let mut best: Option<(i32, usize, String)> = None;
    for definition in parsed.definitions.iter().filter(|definition| {
        matches!(definition.kind.as_str(), "function" | "method")
    }) {
        let score = score_anchor_candidate(parsed, definition, &topic_lower, role);
        if score <= 0 {
            continue;
        }
        let candidate = (score, definition.line, definition.name.clone());
        if best.as_ref().is_none_or(|(best_score, _, _)| candidate.0 > *best_score) {
            best = Some(candidate);
        }
    }

    best.map(|(_, line, name)| (line, name))
}

fn score_anchor_candidate(
    parsed: &ParsedFile,
    definition: &Definition,
    topic: &str,
    role: &str,
) -> i32 {
    let name = definition.name.to_lowercase();
    let mut score = 0;

    if name.starts_with('_') {
        return 0;
    }

    if has_nearby_route(parsed, definition.line, topic) {
        score += 220;
    }

    if name == topic {
        score += 160;
    } else if name.contains(topic) {
        score += 90;
    }

    for alias in topic_aliases(topic) {
        if name == alias {
            score += 140;
        } else if name.contains(&alias) {
            score += 80;
        }
    }

    if name.ends_with("_page") || name.ends_with("_handler") {
        score += 45;
    }

    if name.ends_with("_gate") || name.contains("_verify") {
        score += 35;
    }

    if name.contains("_rate_limit")
        || name.contains("_pre_")
        || name.contains("_blacklist_block")
        || name.contains("build_mock")
    {
        score -= 120;
    }

    let role_lower = role.to_lowercase();
    if (role_lower.contains("entrypoint") || role_lower.contains("http") || role_lower.contains("route"))
        && (name.contains("register")
            || name.contains("verify")
            || name.contains("login")
            || name.contains("checkin")
            || name.contains("checkout"))
    {
        score += 40;
    }

    score
}

fn topic_aliases(topic: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    if topic.contains("registration") {
        aliases.push("register".to_string());
        aliases.push("verify".to_string());
    }
    if topic.contains("auth") {
        aliases.push("login".to_string());
        aliases.push("logout".to_string());
    }
    if topic.contains("gate") {
        aliases.push("checkin".to_string());
        aliases.push("checkout".to_string());
    }
    aliases
}

fn has_nearby_route(parsed: &ParsedFile, function_line: usize, topic: &str) -> bool {
    parsed
        .definitions
        .iter()
        .filter(|definition| definition.kind == "route" && definition.line < function_line)
        .filter(|route| function_line.saturating_sub(route.line) <= 6)
        .any(|route| route_matches_topic(&route.name, topic))
}

fn route_matches_topic(route_name: &str, topic: &str) -> bool {
    let route_lower = route_name.to_lowercase();
    if route_lower.contains(topic) {
        return true;
    }
    for alias in topic_aliases(topic) {
        if route_lower.contains(&alias) {
            return true;
        }
    }
    false
}

fn pick_entry_method(parsed: &ParsedFile) -> Option<(usize, String)> {
    for method_name in ENTRY_METHODS {
        if let Some(definition) = parsed
            .definitions
            .iter()
            .find(|definition| definition.name == *method_name)
        {
            return Some((definition.line, definition.name.clone()));
        }
    }

    parsed
        .definitions
        .iter()
        .find(|definition| definition.kind == "function" && definition.name != "__init__")
        .map(|definition| (definition.line, definition.name.clone()))
}

fn pick_method_after_line(parsed: &ParsedFile, class_line: usize) -> Option<(usize, String)> {
    let candidates: Vec<&Definition> = parsed
        .definitions
        .iter()
        .filter(|definition| definition.kind == "function" && definition.line > class_line)
        .collect();

    for method_name in ENTRY_METHODS {
        if let Some(definition) = candidates.iter().find(|definition| definition.name == *method_name)
        {
            return Some((definition.line, definition.name.clone()));
        }
    }

    candidates
        .iter()
        .find(|definition| definition.name != "__init__")
        .map(|definition| (definition.line, definition.name.clone()))
}

fn extract_definition_block(lines: &[&str], start_line: usize) -> Result<FileSnippet, String> {
    let start_index = start_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let base_indent = leading_whitespace(lines[start_index]);
    let mut end_index = start_index;

    for index in (start_index + 1)..lines.len() {
        let line = lines[index];
        if line.trim().is_empty() {
            end_index = index;
            continue;
        }

        let indent = leading_whitespace(line);
        if indent <= base_indent && is_peer_definition(line) {
            break;
        }

        end_index = index;
    }

    let mut truncated = false;
    let line_count = end_index - start_index + 1;
    if line_count > MAX_SNIPPET_LINES {
        end_index = start_index + MAX_SNIPPET_LINES - 1;
        truncated = true;
    }

    Ok(FileSnippet {
        start_line: start_index + 1,
        end_line: end_index + 1,
        lines: lines[start_index..=end_index]
            .iter()
            .map(|line| line.to_string())
            .collect(),
        truncated,
    })
}

fn leading_whitespace(line: &str) -> usize {
    line.chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .count()
}

fn is_peer_definition(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("def ")
        || trimmed.starts_with("async def ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("@")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("func ")
}

fn find_definition_by_name<'a>(parsed: &'a ParsedFile, name: &str) -> Option<&'a Definition> {
    parsed
        .definitions
        .iter()
        .find(|definition| definition.name == name)
}

fn first_non_init_function<'a>(parsed: &'a ParsedFile) -> Option<&'a Definition> {
    parsed.definitions.iter().find(|definition| {
        definition.kind == "function" && definition.name != "__init__"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Definition;

    #[test]
    fn read_snippet_from_demo_fixture_shows_handler_body() {
        let repo = Path::new("tests/fixtures/demo_app");
        let parsed = ParsedFile {
            path: "auth/routes.py".to_string(),
            language: "python".to_string(),
            definitions: vec![Definition {
                kind: "function".to_string(),
                name: "login_handler".to_string(),
                line: 21,
            }],
            imports: Vec::new(),
            calls: Vec::new(),
        };

        let snippet =
            read_definition_snippet(repo, "auth/routes.py", &parsed, "login", "HTTP routes", None)
                .expect("snippet");
        assert!(snippet.lines.iter().any(|line| line.contains("service.login")));
        assert!(snippet.lines.iter().any(|line| line.contains("def login_handler")));
        assert!(!snippet.lines.iter().any(|line| line.contains("register_handler")));
    }

    #[test]
    fn middleware_fixture_prefers_call_over_class() {
        let repo = Path::new("tests/benchmarks/starlette");
        let path = "starlette/middleware/base.py";
        let atlas_dir = repo.join(".atlas");
        let symbols = crate::parse::load_symbols(&atlas_dir).expect("symbols");
        let parsed = symbols
            .files
            .iter()
            .find(|file| file.path.replace('\\', "/") == path)
            .expect("parsed file");

        let anchor = resolve_anchor(parsed, "middleware", "depended on by 3 file(s)", None)
            .expect("anchor");
        assert_eq!(anchor.1, "__call__");

        let snippet = read_definition_snippet(
            repo,
            path,
            parsed,
            "middleware",
            "depended on by 3 file(s)",
            None,
        )
        .expect("snippet");
        assert!(snippet.lines.iter().any(|line| line.contains("async def __call__")));
        assert!(!snippet.lines.iter().any(|line| line.contains("_wrapped_rcv_disconnected")));
    }

    #[test]
    fn errors_snippet_starts_at_class_not_html_template() {
        let repo = Path::new("tests/benchmarks/starlette");
        let path = "starlette/middleware/errors.py";
        let atlas_dir = repo.join(".atlas");
        let symbols = crate::parse::load_symbols(&atlas_dir).expect("symbols");
        let parsed = symbols
            .files
            .iter()
            .find(|file| file.path.replace('\\', "/") == path)
            .expect("parsed file");

        let snippet = read_definition_snippet(
            repo,
            path,
            parsed,
            "middleware",
            "depended on by 2 file(s)",
            Some("ServerErrorMiddleware"),
        )
        .expect("snippet");
        assert!(snippet
            .lines
            .first()
            .expect("first line")
            .contains("async def __call__")
            || snippet.lines.iter().any(|line| line.contains("async def __call__")));
        assert!(!snippet.lines.iter().any(|line| line.contains("<span class=\"lineno\">")));
    }

    #[test]
    fn resolve_anchor_prefers_topic_match() {
        let parsed = ParsedFile {
            path: "auth/routes.py".to_string(),
            language: "python".to_string(),
            definitions: vec![
                Definition {
                    kind: "function".to_string(),
                    name: "health".to_string(),
                    line: 5,
                },
                Definition {
                    kind: "function".to_string(),
                    name: "login_handler".to_string(),
                    line: 21,
                },
            ],
            imports: Vec::new(),
            calls: Vec::new(),
        };

        let anchor = resolve_anchor(&parsed, "login", "HTTP routes", None).expect("anchor");
        assert_eq!(anchor, (21, "login_handler".to_string()));
    }

    #[test]
    fn resolve_anchor_skips_private_registration_helpers() {
        let parsed = ParsedFile {
            path: "registration/app.py".to_string(),
            language: "python".to_string(),
            definitions: vec![
                Definition {
                    kind: "function".to_string(),
                    name: "_registration_pre_rate_limit".to_string(),
                    line: 166,
                },
                Definition {
                    kind: "route".to_string(),
                    name: "/register".to_string(),
                    line: 893,
                },
                Definition {
                    kind: "function".to_string(),
                    name: "register_page".to_string(),
                    line: 894,
                },
                Definition {
                    kind: "route".to_string(),
                    name: "/verify-face".to_string(),
                    line: 2012,
                },
                Definition {
                    kind: "function".to_string(),
                    name: "verify_face".to_string(),
                    line: 2013,
                },
            ],
            imports: Vec::new(),
            calls: Vec::new(),
        };

        let anchor =
            resolve_anchor(&parsed, "registration", "entrypoint", None).expect("anchor");
        assert_eq!(anchor.1, "register_page");
    }
}
