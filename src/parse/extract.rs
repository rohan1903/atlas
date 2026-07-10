use std::path::Path;

use tree_sitter::{Node, Parser};

use super::language::LanguageKind;
use super::{Call, Definition, Import, ParsedFile};

const MAX_PARSE_BYTES: usize = 5 * 1024 * 1024;

pub fn parse_file(path: &str, language: LanguageKind, source: &str) -> Option<ParsedFile> {
    if source.len() > MAX_PARSE_BYTES {
        return None;
    }

    let file_path = Path::new(path);
    let mut parser = Parser::new();
    parser
        .set_language(&language.tree_sitter_language(file_path))
        .ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut definitions = Vec::new();
    let mut imports = Vec::new();
    let mut calls = Vec::new();

    walk_node(
        root,
        source,
        language,
        &mut definitions,
        &mut imports,
        &mut calls,
    );

    Some(ParsedFile {
        path: path.to_string(),
        language: language.as_str().to_string(),
        definitions,
        imports,
        calls,
    })
}

fn walk_node(
    node: Node,
    source: &str,
    language: LanguageKind,
    definitions: &mut Vec<Definition>,
    imports: &mut Vec<Import>,
    calls: &mut Vec<Call>,
) {
    collect_from_node(node, source, language, definitions, imports, calls);

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            walk_node(child, source, language, definitions, imports, calls);
        }
    }
}

fn collect_from_node(
    node: Node,
    source: &str,
    language: LanguageKind,
    definitions: &mut Vec<Definition>,
    imports: &mut Vec<Import>,
    calls: &mut Vec<Call>,
) {
    let line = node.start_position().row + 1;

    match node.kind() {
        "function_definition" | "function_declaration" | "method_declaration" | "function_item" => {
            if let Some(name) = definition_name(node, source) {
                definitions.push(Definition {
                    kind: if node.kind() == "method_declaration" {
                        "method".to_string()
                    } else {
                        "function".to_string()
                    },
                    name,
                    line,
                });
            }
            collect_route_decorators(node, source, definitions);
        }
        "class_definition" | "class_declaration" => {
            if let Some(name) = definition_name(node, source) {
                definitions.push(Definition {
                    kind: "class".to_string(),
                    name,
                    line,
                });
            }
        }
        "struct_specifier" | "enum_specifier" | "struct_item" | "enum_item" => {
            if let Some(name) = definition_name(node, source) {
                let kind = match node.kind() {
                    "struct_item" => "struct",
                    "enum_item" => "enum",
                    other => other.trim_end_matches("_specifier"),
                };
                definitions.push(Definition {
                    kind: kind.to_string(),
                    name,
                    line,
                });
            }
        }
        "type_declaration" if language == LanguageKind::Go => {
            if let Some(name) = definition_name(node, source) {
                definitions.push(Definition {
                    kind: "type".to_string(),
                    name,
                    line,
                });
            }
        }
        "import_statement" | "import_from_statement" => {
            if let Some(target) = import_target(node, source, language) {
                imports.push(Import {
                    kind: "import".to_string(),
                    target,
                    line,
                });
            }
        }
        "preproc_include" => {
            if let Some(target) = include_target(node, source) {
                imports.push(Import {
                    kind: "include".to_string(),
                    target,
                    line,
                });
            }
        }
        "import_declaration" if language == LanguageKind::Go => {
            for index in 0..node.child_count() {
                if let Some(child) = node.child(index) {
                    if child.kind() == "import_spec" || child.kind() == "import_spec_list" {
                        collect_go_import(child, source, imports, line);
                    }
                }
            }
        }
        "use_declaration" if language == LanguageKind::Rust => {
            if let Some(target) = rust_use_target(node, source) {
                imports.push(Import {
                    kind: "use".to_string(),
                    target,
                    line,
                });
            }
        }
        "mod_item" if language == LanguageKind::Rust => {
            if let Some(name) = definition_name(node, source) {
                imports.push(Import {
                    kind: "mod".to_string(),
                    target: name,
                    line,
                });
            }
        }
        "call_expression" | "call" => {
            if let Some(target) = call_target(node, source) {
                calls.push(Call { target, line });
            }
            if let Some(route) = detect_http_route_call(node, source) {
                definitions.push(Definition {
                    kind: "route".to_string(),
                    name: route,
                    line,
                });
            }
        }
        _ => {}
    }
}

fn collect_go_import(node: Node, source: &str, imports: &mut Vec<Import>, line: usize) {
    if node.kind() == "import_spec" {
        if let Some(target) = node
            .child_by_field_name("path")
            .map(|n| node_text(n, source).trim_matches('"').to_string())
        {
            imports.push(Import {
                kind: "import".to_string(),
                target,
                line,
            });
        }
        return;
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            collect_go_import(child, source, imports, line);
        }
    }
}

fn definition_name(node: Node, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return Some(node_text(name_node, source).to_string());
    }

    if let Some(declarator) = node.child_by_field_name("declarator") {
        if let Some(name) = identifier_from_declarator(declarator, source) {
            return Some(name);
        }
    }

    first_identifier(node, source)
}

fn identifier_from_declarator(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" | "field_identifier" => {
            Some(node_text(node, source).to_string())
        }
        "function_declarator" | "pointer_declarator" | "parenthesized_declarator" => {
            if let Some(inner) = node.child_by_field_name("declarator") {
                return identifier_from_declarator(inner, source);
            }
            first_identifier(node, source)
        }
        _ => first_identifier(node, source),
    }
}

fn first_identifier(node: Node, source: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier"
    ) {
        return Some(node_text(node, source).to_string());
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if let Some(name) = first_identifier(child, source) {
                return Some(name);
            }
        }
    }

    None
}

fn import_target(node: Node, source: &str, language: LanguageKind) -> Option<String> {
    match language {
        LanguageKind::Python => {
            if node.kind() == "import_from_statement" {
                if let Some(module) = node.child_by_field_name("module_name") {
                    return Some(node_text(module, source).to_string());
                }
            }
            if let Some(name) = node.child_by_field_name("name") {
                return Some(
                    node_text(name, source)
                        .trim_matches(&['"', '\''][..])
                        .to_string(),
                );
            }
            if let Some(module) = node.child_by_field_name("module_name") {
                return Some(node_text(module, source).to_string());
            }
        }
        LanguageKind::JavaScript | LanguageKind::TypeScript => {
            if let Some(source_node) = node.child_by_field_name("source") {
                return Some(
                    node_text(source_node, source)
                        .trim_matches(&['"', '\''][..])
                        .to_string(),
                );
            }
        }
        LanguageKind::Rust => return rust_use_target(node, source),
        _ => {}
    }

    first_string_like(node, source)
}

fn rust_use_target(node: Node, source: &str) -> Option<String> {
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if matches!(
                child.kind(),
                "scoped_identifier"
                    | "identifier"
                    | "use_list"
                    | "use_as_clause"
                    | "crate"
                    | "self"
                    | "super"
            ) {
                let target = node_text(child, source)
                    .trim()
                    .trim_end_matches(';')
                    .to_string();
                if !target.is_empty() {
                    return Some(target);
                }
            }
        }
    }

    let text = node_text(node, source).trim();
    text.strip_prefix("use ")
        .map(|target| target.trim().trim_end_matches(';').to_string())
        .filter(|target| !target.is_empty())
}

fn include_target(node: Node, source: &str) -> Option<String> {
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            match child.kind() {
                "string_literal" | "system_lib_string" => {
                    return Some(
                        node_text(child, source)
                            .trim_matches(&['"', '<', '>', '\''][..])
                            .to_string(),
                    );
                }
                _ => {}
            }
        }
    }

    None
}

fn call_target(node: Node, source: &str) -> Option<String> {
    if let Some(function) = node.child_by_field_name("function") {
        return Some(callable_text(function, source));
    }

    if let Some(child) = node.named_child(0) {
        return Some(callable_text(child, source));
    }

    None
}

fn callable_text(node: Node, source: &str) -> String {
    match node.kind() {
        "identifier" | "type_identifier" | "field_identifier" => {
            node_text(node, source).to_string()
        }
        "member_expression" | "field_expression" | "selector_expression" => {
            if let Some(property) = node.child_by_field_name("property") {
                node_text(property, source).to_string()
            } else if let Some(field) = node.child_by_field_name("field") {
                node_text(field, source).to_string()
            } else {
                node_text(node, source).to_string()
            }
        }
        "scoped_identifier" => node_text(node, source).to_string(),
        "generic_function" => node
            .child_by_field_name("function")
            .map(|function| callable_text(function, source))
            .unwrap_or_else(|| node_text(node, source).to_string()),
        _ => node_text(node, source).to_string(),
    }
}

fn first_string_like(node: Node, source: &str) -> Option<String> {
    if matches!(node.kind(), "string" | "string_literal") {
        return Some(
            node_text(node, source)
                .trim_matches(&['"', '\''][..])
                .to_string(),
        );
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if let Some(value) = first_string_like(child, source) {
                return Some(value);
            }
        }
    }

    None
}

fn detect_http_route_call(node: Node, source: &str) -> Option<String> {
    const HTTP_METHODS: &[&str] = &[
        "get", "post", "put", "patch", "delete", "options", "head", "route",
    ];

    let callee = node
        .child_by_field_name("function")
        .or_else(|| node.named_child(0))?;
    let method = callable_text(callee, source).to_lowercase();
    if !HTTP_METHODS
        .iter()
        .any(|verb| method == *verb || method.ends_with(&format!(".{verb}")))
    {
        return None;
    }

    let route = first_string_like(node, source)?;
    if route.contains('/') {
        Some(format!("{method} {route}"))
    } else {
        None
    }
}

fn collect_route_decorators(node: Node, source: &str, definitions: &mut Vec<Definition>) {
    for index in 0..node.child_count() {
        let Some(child) = node.child(index) else {
            continue;
        };
        if child.kind() != "decorator" {
            continue;
        }
        let text = node_text(child, source).to_lowercase();
        if !text.contains("route") && !text.contains("get") && !text.contains("post") {
            continue;
        }
        if let Some(route) = first_string_like(child, source) {
            if route.contains('/') {
                definitions.push(Definition {
                    kind: "route".to_string(),
                    name: route,
                    line: child.start_position().row + 1,
                });
            }
        }
    }
}

fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::language::LanguageKind;

    #[test]
    fn parses_c_function_and_include() {
        let source = r#"
#include <linux/sched.h>

int schedule(void) {
    printk("sched");
    return 0;
}
"#;
        let parsed = parse_file("sched.c", LanguageKind::C, source).expect("parse C");
        assert!(parsed
            .imports
            .iter()
            .any(|i| i.target.contains("linux/sched.h")));
        assert!(parsed.definitions.iter().any(|d| d.name == "schedule"));
        assert!(parsed.calls.iter().any(|c| c.target == "printk"));
    }

    #[test]
    fn parses_python_import_and_call() {
        let source = r#"
from auth.service import AuthService

def login():
    AuthService.authenticate()
"#;
        let parsed = parse_file("auth.py", LanguageKind::Python, source).expect("parse python");
        assert!(parsed.imports.iter().any(|i| i.target.contains("auth")));
        assert!(parsed.definitions.iter().any(|d| d.name == "login"));
        assert!(parsed
            .calls
            .iter()
            .any(|c| c.target.contains("authenticate")));
    }

    #[test]
    fn parses_rust_items_imports_and_calls() {
        let source = r#"
use crate::graph::rank_files;
mod scan;

pub struct Cli {
    value: usize,
}

impl Cli {
    pub fn run(&self) {
        rank_files();
        scan::run();
    }
}

fn main() {
    Cli { value: 1 }.run();
}
"#;
        let parsed = parse_file("src/main.rs", LanguageKind::Rust, source).expect("parse rust");
        assert!(parsed
            .imports
            .iter()
            .any(|i| i.target.contains("crate::graph::rank_files")));
        assert!(parsed.imports.iter().any(|i| i.target == "scan"));
        assert!(parsed
            .definitions
            .iter()
            .any(|d| d.kind == "struct" && d.name == "Cli"));
        assert!(parsed.definitions.iter().any(|d| d.name == "run"));
        assert!(parsed.definitions.iter().any(|d| d.name == "main"));
        assert!(parsed.calls.iter().any(|c| c.target.contains("rank_files")));
        assert!(parsed.calls.iter().any(|c| c.target.contains("scan::run")));
    }
}
