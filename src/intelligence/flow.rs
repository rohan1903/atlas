use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::intelligence::architecture::subsystem_key;
use crate::parse::{Definition, Import, ParseOutput, ParsedFile};
use crate::paths;

const MAX_DEPTH: usize = 12;
const MAX_STEPS: usize = 24;
const COMPRESSED_FLOW_LIMIT: usize = 8;

#[derive(Debug, Clone)]
pub struct FlowStep {
    pub label: String,
    #[allow(dead_code)]
    pub kind: String,
    pub file: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FlowResult {
    pub query: String,
    pub seed: String,
    pub steps: Vec<FlowStep>,
}

#[derive(Debug, Clone)]
struct Seed {
    file: String,
    definition: Definition,
    score: i32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SymbolRef {
    file: String,
    name: String,
    line: usize,
}

pub fn extract_flow(repo: &Path, query: &str) -> Result<FlowResult, String> {
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;
    let normalized_query = query.trim().to_lowercase();

    if normalized_query.is_empty() {
        return Err("flow name cannot be empty".to_string());
    }

    let index = DefinitionIndex::from_symbols(&symbols);
    let mut seeds = find_seeds(&symbols, &normalized_query);
    if seeds.is_empty() {
        return Err(format!(
            "no flow seed found for '{query}' — try a function, route, or file name from the scan"
        ));
    }

    seeds.sort_by_key(|seed| std::cmp::Reverse(seed.score));

    let mut best: Option<(Seed, Vec<FlowStep>)> = None;
    for seed in seeds {
        let steps = trace_flow(&seed, &symbols, &index);
        if steps.len() <= 1 {
            continue;
        }
        best = Some((seed, steps));
        break;
    }

    let Some((best_seed, steps)) = best else {
        return Err(format!(
            "seed found for '{query}' but no downstream calls were resolved — graph may be incomplete"
        ));
    };

    Ok(FlowResult {
        query: query.to_string(),
        seed: best_seed.definition.name.clone(),
        steps,
    })
}

pub fn compress_flow_steps(steps: &[FlowStep], verbose: bool) -> Vec<FlowStep> {
    if verbose || steps.len() <= 2 {
        return steps.to_vec();
    }

    let mut compressed: Vec<FlowStep> = steps
        .iter()
        .filter(|step| is_primary_flow_step(&step.label, &step.file, &step.kind))
        .cloned()
        .collect();

    if compressed.len() < 2 {
        compressed = steps
            .iter()
            .filter(|step| !is_supporting_flow_step(&step.label, &step.file, &step.kind))
            .cloned()
            .collect();
    }

    if compressed.len() < 2 {
        return cap_flow_steps(steps, COMPRESSED_FLOW_LIMIT.min(steps.len().max(2)));
    }

    cap_flow_steps(&compressed, COMPRESSED_FLOW_LIMIT)
}

fn cap_flow_steps(steps: &[FlowStep], limit: usize) -> Vec<FlowStep> {
    steps.iter().take(limit).cloned().collect()
}

pub fn flow_subsystem_score(flow: &FlowResult, target_key: &str) -> i32 {
    let Some(first) = flow.steps.first() else {
        return 0;
    };

    if subsystem_key(&first.file) != target_key {
        return 0;
    }

    let in_subsystem = flow
        .steps
        .iter()
        .filter(|step| subsystem_key(&step.file) == target_key)
        .count();

    let mut score = 300 + in_subsystem as i32 * 40;
    score += (24_i32).saturating_sub(flow.steps.len() as i32);
    score
}

fn is_primary_flow_step(name: &str, file: &str, kind: &str) -> bool {
    if is_flow_noise(name, file, kind) || is_supporting_flow_step(name, file, kind) {
        return false;
    }

    let name_lower = name.to_lowercase();
    name_lower.contains("process_")
        || name_lower.contains("validate_")
        || name_lower.contains("parse_")
        || name_lower.contains("verify")
        || name_lower.contains("register")
        || name_lower.contains("login")
        || name_lower.contains("logout")
        || name_lower.contains("checkin")
        || name_lower.contains("checkout")
        || name_lower.contains("create_")
        || name_lower.contains("handle_")
        || name_lower.ends_with("_handler")
        || name_lower.ends_with("_gate")
        || name_lower.contains("_verify_and_")
        || name_lower.contains("get_by_")
        || name_lower.contains("record_")
        || name_lower.contains("generate_")
        || name_lower.contains("invalidate_")
        || name_lower.contains("update_qr")
        || name_lower.contains("get_face")
        || name_lower.contains("find_all_face")
        || name_lower.contains("detect_twin")
}

fn is_supporting_flow_step(name: &str, file: &str, kind: &str) -> bool {
    if is_flow_noise(name, file, kind) {
        return true;
    }

    let name_lower = name.to_lowercase();
    if is_db_mock_helper(&name_lower) {
        return true;
    }

    (name_lower.starts_with("send_") && name_lower.ends_with("_email"))
        || name_lower == "l2_distance"
        || name_lower == "absolute_feedback_link"
        || name_lower == "send_exceeded_email"
        || name_lower == "process_checkout"
        || name_lower == "collect_department_choices"
}

fn is_db_mock_helper(name: &str) -> bool {
    matches!(
        name,
        "get" | "set" | "child" | "update" | "db_reference" | "get_or_create"
    )
}

fn trace_flow(seed: &Seed, symbols: &ParseOutput, index: &DefinitionIndex) -> Vec<FlowStep> {
    let mut steps = Vec::new();
    let mut visited: HashSet<SymbolRef> = HashSet::new();
    let mut queue = VecDeque::new();

    let start = SymbolRef {
        file: seed.file.clone(),
        name: seed.definition.name.clone(),
        line: seed.definition.line,
    };

    steps.push(FlowStep {
        label: seed.definition.name.clone(),
        kind: seed.definition.kind.clone(),
        file: seed.file.clone(),
        line: Some(seed.definition.line),
    });
    visited.insert(start.clone());
    queue.push_back((start, 0));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= MAX_DEPTH || steps.len() >= MAX_STEPS {
            break;
        }

        let Some(parsed) = symbols.files.iter().find(|file| file.path == current.file) else {
            continue;
        };

        let call_targets = calls_for_symbol(parsed, &current);
        for target in call_targets {
            let Some(next) = index.resolve_call(&target, Some(parsed)) else {
                continue;
            };

            if is_flow_noise(&next.name, &next.file, &next.kind) {
                continue;
            }

            let next_ref = SymbolRef {
                file: next.file.clone(),
                name: next.name.clone(),
                line: next.line,
            };
            if visited.contains(&next_ref) {
                continue;
            }

            visited.insert(next_ref.clone());
            steps.push(FlowStep {
                label: next.name.clone(),
                kind: next.kind.clone(),
                file: next.file.clone(),
                line: Some(next.line),
            });
            queue.push_back((next_ref, depth + 1));
        }
    }

    steps
}

fn calls_for_symbol(parsed: &ParsedFile, current: &SymbolRef) -> Vec<String> {
    let end_line = next_definition_line(parsed, current.line);
    parsed
        .calls
        .iter()
        .filter(|call| call.line >= current.line && call.line < end_line)
        .map(|call| call.target.clone())
        .collect()
}

fn next_definition_line(parsed: &ParsedFile, line: usize) -> usize {
    parsed
        .definitions
        .iter()
        .map(|definition| definition.line)
        .filter(|definition_line| *definition_line > line)
        .min()
        .unwrap_or(usize::MAX)
}

fn find_seeds(symbols: &ParseOutput, query: &str) -> Vec<Seed> {
    let mut seeds = Vec::new();

    for parsed in &symbols.files {
        for definition in &parsed.definitions {
            let score = score_seed(parsed, definition, query);
            if score > 0 {
                seeds.push(Seed {
                    file: parsed.path.clone(),
                    definition: definition.clone(),
                    score,
                });
            }
        }
    }

    seeds
}

fn score_seed(parsed: &ParsedFile, definition: &Definition, query: &str) -> i32 {
    if definition.kind == "route" {
        return 0;
    }

    let name = definition.name.to_lowercase();
    let path = parsed.path.to_lowercase();
    let mut score = 0;

    if name == query {
        score += 120;
    } else if name.contains(query) {
        score += 70;
    }

    if path.contains(query) {
        score += 25;
    }

    if name == format!("{query}_handler") || name == format!("handle_{query}") {
        score += 100;
    }

    if name.ends_with("_success")
        || name.ends_with("_page")
        || name.ends_with("_template")
        || name.ends_with("_backup")
    {
        score -= 100;
    }

    if name == format!("{query}_gate") {
        score += 120;
    } else if name.ends_with("_gate") {
        score += 70;
    } else if name.contains("_verify_and_") {
        score += 25;
    } else if name.ends_with("_verify") {
        score += 40;
    }

    if name.starts_with(&format!("{query}_")) && !name.ends_with("_success") {
        score += 35;
    }

    if path.contains("routes") && name.contains(query) {
        score += 50;
    }

    if name == query && path.contains("service") {
        score -= 30;
    }

    if paths::is_test_path(&parsed.path) || name.starts_with("test_") {
        score -= 200;
    }

    if !paths::is_test_path(&parsed.path) && !paths::is_excluded_from_clustering(&parsed.path) {
        score += 30;
    }

    if matches!(definition.kind.as_str(), "function" | "method") {
        score += 10;
    }

    score
}

#[derive(Clone)]
struct DefinitionRecord {
    name: String,
    kind: String,
    file: String,
    line: usize,
}

struct DefinitionIndex {
    by_name: HashMap<String, Vec<DefinitionRecord>>,
}

impl DefinitionIndex {
    fn from_symbols(symbols: &ParseOutput) -> Self {
        let mut by_name: HashMap<String, Vec<DefinitionRecord>> = HashMap::new();
        for parsed in &symbols.files {
            for definition in &parsed.definitions {
                by_name
                    .entry(definition.name.to_lowercase())
                    .or_default()
                    .push(DefinitionRecord {
                        name: definition.name.clone(),
                        kind: definition.kind.clone(),
                        file: parsed.path.clone(),
                        line: definition.line,
                    });
            }
        }
        Self { by_name }
    }

    fn resolve_call(&self, target: &str, caller: Option<&ParsedFile>) -> Option<DefinitionRecord> {
        for key in call_lookup_keys(target) {
            if let Some(record) = self.pick_candidate(&key, caller) {
                if record.kind == "class" {
                    return None;
                }
                return Some(record);
            }
        }
        None
    }

    fn pick_candidate(&self, key: &str, caller: Option<&ParsedFile>) -> Option<DefinitionRecord> {
        let candidates = self.by_name.get(key)?;
        let caller_file = caller.map(|parsed| parsed.path.as_str());
        let caller_imports = caller
            .map(|parsed| parsed.imports.as_slice())
            .unwrap_or(&[]);

        if let Some(file) = caller_file {
            if let Some(local) = candidates
                .iter()
                .filter(|record| record.file == file)
                .min_by_key(|record| record.line)
            {
                return Some(local.clone());
            }
        }

        let best = candidates
            .iter()
            .max_by_key(|record| score_call_candidate(record, caller_file, caller_imports))
            .cloned();

        best.filter(|record| score_call_candidate(record, caller_file, caller_imports) > -5_000)
    }
}

fn score_call_candidate(
    record: &DefinitionRecord,
    caller_file: Option<&str>,
    caller_imports: &[Import],
) -> i32 {
    let mut score = 0;

    if let Some(caller) = caller_file {
        if subsystem_key(&record.file) == subsystem_key(caller) {
            score += 200;
        }

        if !paths::is_test_path(caller) && paths::is_test_path(&record.file) {
            score -= 10_000;
        }
    }

    for import in caller_imports {
        if import_matches_file(&import.target, &record.file) {
            score += 500;
        }
    }

    if paths::is_deprioritized_path(&record.file) {
        score -= 600;
    }

    score -= (record.line as i32).min(99);
    score
}

fn import_matches_file(import_target: &str, file_path: &str) -> bool {
    let module = import_target.trim().replace('.', "/");
    if module.is_empty() {
        return false;
    }

    let normalized = paths::normalize_path(file_path);
    normalized == format!("{module}.py") || normalized.ends_with(&format!("/{module}.py"))
}

fn is_flow_noise(name: &str, file: &str, kind: &str) -> bool {
    if kind == "class" {
        return true;
    }

    let name_lower = name.to_lowercase();
    let file_lower = file.replace('\\', "/").to_lowercase();

    if name_lower == "__init__" {
        return true;
    }

    if name_lower.starts_with("log_")
        || matches!(
            name_lower.as_str(),
            "log_info"
                | "log_error"
                | "log_debug"
                | "log_warn"
                | "log_warning"
                | "setup_logging"
                | "println"
                | "print"
                | "printf"
                | "fprintf"
                | "debug_print"
        )
    {
        return true;
    }

    if name.starts_with('_') && !name.starts_with("__") {
        return true;
    }

    if file_lower.contains("/utils/") || file_lower.starts_with("utils/") {
        return true;
    }

    if (file_lower.contains("/config/") || file_lower.starts_with("config/"))
        && matches!(
            name_lower.as_str(),
            "get_database_url" | "load_settings" | "get_setting" | "get"
        )
    {
        return true;
    }

    false
}

fn call_lookup_keys(target: &str) -> Vec<String> {
    let normalized = target.trim().to_lowercase();
    let mut keys = Vec::new();

    if let Some((_, method)) = normalized.rsplit_once('.') {
        if !method.is_empty() {
            keys.push(method.to_string());
        }
    }

    keys.push(normalized);
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{Call, Import, ParseOutput, ParseSummary};

    fn sample_symbols() -> ParseOutput {
        ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "core.c".to_string(),
                    language: "c".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "core_init".to_string(),
                        line: 3,
                    }],
                    imports: vec![Import {
                        kind: "include".to_string(),
                        target: "util.h".to_string(),
                        line: 1,
                    }],
                    calls: vec![Call {
                        target: "helper".to_string(),
                        line: 4,
                    }],
                },
                ParsedFile {
                    path: "util.c".to_string(),
                    language: "c".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "helper".to_string(),
                        line: 3,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        }
    }

    #[test]
    fn traces_call_chain_for_seed() {
        let symbols = sample_symbols();
        let index = DefinitionIndex::from_symbols(&symbols);
        let seeds = find_seeds(&symbols, "core");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        let steps = trace_flow(&seed, &symbols, &index);
        assert!(steps.iter().any(|step| step.label == "core_init"));
        assert!(steps.iter().any(|step| step.label == "helper"));
    }

    #[test]
    fn resolves_dotted_call_targets() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "auth/service.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![
                        Definition {
                            kind: "function".to_string(),
                            name: "login".to_string(),
                            line: 5,
                        },
                        Definition {
                            kind: "function".to_string(),
                            name: "get_by_email".to_string(),
                            line: 20,
                        },
                    ],
                    imports: vec![],
                    calls: vec![
                        Call {
                            target: "self.user_repo.get_by_email".to_string(),
                            line: 6,
                        },
                        Call {
                            target: "log_info".to_string(),
                            line: 7,
                        },
                    ],
                },
                ParsedFile {
                    path: "utils/logger.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "log_info".to_string(),
                        line: 1,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        };

        let index = DefinitionIndex::from_symbols(&symbols);
        let keys = call_lookup_keys("self.user_repo.get_by_email");
        assert!(keys.contains(&"get_by_email".to_string()));

        let seeds = find_seeds(&symbols, "login");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        let steps = trace_flow(&seed, &symbols, &index);
        assert!(steps.iter().any(|step| step.label == "get_by_email"));
        assert!(!steps.iter().any(|step| step.label == "log_info"));
    }

    #[test]
    fn skips_init_and_private_helpers() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![ParsedFile {
                path: "auth/repository.py".to_string(),
                language: "python".to_string(),
                definitions: vec![
                    Definition {
                        kind: "function".to_string(),
                        name: "verify_password".to_string(),
                        line: 10,
                    },
                    Definition {
                        kind: "function".to_string(),
                        name: "_fetch_hash".to_string(),
                        line: 20,
                    },
                ],
                imports: vec![],
                calls: vec![Call {
                    target: "_fetch_hash".to_string(),
                    line: 11,
                }],
            }],
        };

        let index = DefinitionIndex::from_symbols(&symbols);
        let seed = Seed {
            file: "auth/repository.py".to_string(),
            definition: symbols.files[0].definitions[0].clone(),
            score: 100,
        };
        let steps = trace_flow(&seed, &symbols, &index);
        assert_eq!(steps.len(), 1);
        assert!(!steps.iter().any(|step| step.label == "_fetch_hash"));
    }

    #[test]
    fn handler_seed_reaches_service_method_without_init() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "auth/routes.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login_handler".to_string(),
                        line: 3,
                    }],
                    imports: vec![],
                    calls: vec![
                        Call {
                            target: "AuthService".to_string(),
                            line: 4,
                        },
                        Call {
                            target: "log_info".to_string(),
                            line: 5,
                        },
                        Call {
                            target: "service.login".to_string(),
                            line: 6,
                        },
                    ],
                },
                ParsedFile {
                    path: "auth/service.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![
                        Definition {
                            kind: "class".to_string(),
                            name: "AuthService".to_string(),
                            line: 1,
                        },
                        Definition {
                            kind: "function".to_string(),
                            name: "__init__".to_string(),
                            line: 2,
                        },
                        Definition {
                            kind: "function".to_string(),
                            name: "login".to_string(),
                            line: 8,
                        },
                    ],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        };

        let index = DefinitionIndex::from_symbols(&symbols);
        let seeds = find_seeds(&symbols, "login");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        let steps = trace_flow(&seed, &symbols, &index);
        let labels: Vec<_> = steps.iter().map(|step| step.label.as_str()).collect();
        assert_eq!(labels, vec!["login_handler", "login"]);
    }

    #[test]
    fn prefers_handler_over_service_method() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "auth/routes.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login_handler".to_string(),
                        line: 3,
                    }],
                    imports: vec![],
                    calls: vec![Call {
                        target: "service.login".to_string(),
                        line: 4,
                    }],
                },
                ParsedFile {
                    path: "auth/service.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login".to_string(),
                        line: 8,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        };

        let seeds = find_seeds(&symbols, "login");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        assert_eq!(seed.definition.name, "login_handler");
    }

    #[test]
    fn prefers_gate_handler_over_success_template() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![ParsedFile {
                path: "gate/app.py".to_string(),
                language: "python".to_string(),
                definitions: vec![
                    Definition {
                        kind: "function".to_string(),
                        name: "checkin_gate".to_string(),
                        line: 705,
                    },
                    Definition {
                        kind: "function".to_string(),
                        name: "checkin_verify_and_log".to_string(),
                        line: 964,
                    },
                    Definition {
                        kind: "function".to_string(),
                        name: "checkin_success".to_string(),
                        line: 1762,
                    },
                ],
                imports: vec![],
                calls: vec![
                    Call {
                        target: "checkin_verify_and_log".to_string(),
                        line: 706,
                    },
                    Call {
                        target: "get".to_string(),
                        line: 1763,
                    },
                ],
            }],
        };

        let seeds = find_seeds(&symbols, "checkin");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        assert_eq!(seed.definition.name, "checkin_gate");
    }

    #[test]
    fn prefers_imported_module_over_legacy_duplicate() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "auth/routes.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login_handler".to_string(),
                        line: 12,
                    }],
                    imports: vec![Import {
                        kind: "import".to_string(),
                        target: "auth.service".to_string(),
                        line: 3,
                    }],
                    calls: vec![Call {
                        target: "service.login".to_string(),
                        line: 18,
                    }],
                },
                ParsedFile {
                    path: "auth/service.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login".to_string(),
                        line: 16,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
                ParsedFile {
                    path: "legacy_final/auth_service_final_v2.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "login".to_string(),
                        line: 5,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        };

        let index = DefinitionIndex::from_symbols(&symbols);
        let seeds = find_seeds(&symbols, "login");
        let seed = seeds.into_iter().max_by_key(|seed| seed.score).unwrap();
        let steps = trace_flow(&seed, &symbols, &index);
        let labels: Vec<_> = steps.iter().map(|step| step.label.as_str()).collect();
        assert_eq!(labels, vec!["login_handler", "login"]);
        assert_eq!(steps[1].file, "auth/service.py");
    }

    #[test]
    fn production_flow_does_not_resolve_into_tests() {
        let symbols = ParseOutput {
            version: 1,
            summary: ParseSummary::default(),
            files: vec![
                ParsedFile {
                    path: "fastapi/routing.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "app".to_string(),
                        line: 10,
                    }],
                    imports: vec![],
                    calls: vec![Call {
                        target: "f".to_string(),
                        line: 11,
                    }],
                },
                ParsedFile {
                    path: "tests/test_routing.py".to_string(),
                    language: "python".to_string(),
                    definitions: vec![Definition {
                        kind: "function".to_string(),
                        name: "f".to_string(),
                        line: 3,
                    }],
                    imports: vec![],
                    calls: vec![],
                },
            ],
        };

        let index = DefinitionIndex::from_symbols(&symbols);
        let seed = Seed {
            file: "fastapi/routing.py".to_string(),
            definition: symbols.files[0].definitions[0].clone(),
            score: 100,
        };
        let steps = trace_flow(&seed, &symbols, &index);
        assert_eq!(steps.len(), 1);
        assert!(!steps.iter().any(|step| paths::is_test_path(&step.file)));
    }

    #[test]
    fn compresses_noisy_checkin_chain() {
        let steps = vec![
            FlowStep {
                label: "checkin_verify_and_log".to_string(),
                kind: "function".to_string(),
                file: "gate/app.py".to_string(),
                line: Some(964),
            },
            FlowStep {
                label: "get".to_string(),
                kind: "function".to_string(),
                file: "gate/app.py".to_string(),
                line: Some(106),
            },
            FlowStep {
                label: "parse_qr_payload".to_string(),
                kind: "function".to_string(),
                file: "gate/qr_module.py".to_string(),
                line: Some(159),
            },
            FlowStep {
                label: "validate_qr_token".to_string(),
                kind: "function".to_string(),
                file: "gate/qr_module.py".to_string(),
                line: Some(177),
            },
            FlowStep {
                label: "child".to_string(),
                kind: "function".to_string(),
                file: "gate/app.py".to_string(),
                line: Some(87),
            },
            FlowStep {
                label: "process_checkin".to_string(),
                kind: "function".to_string(),
                file: "gate/app.py".to_string(),
                line: Some(1509),
            },
            FlowStep {
                label: "l2_distance".to_string(),
                kind: "function".to_string(),
                file: "gate/app.py".to_string(),
                line: Some(266),
            },
        ];

        let compressed = compress_flow_steps(&steps, false);
        assert!(compressed.len() < steps.len());
        assert!(!compressed.iter().any(|step| step.label == "get"));
        assert!(!compressed.iter().any(|step| step.label == "child"));
        assert!(!compressed.iter().any(|step| step.label == "l2_distance"));
        assert_eq!(
            compressed.first().map(|step| step.label.as_str()),
            Some("checkin_verify_and_log")
        );
        assert!(compressed
            .iter()
            .any(|step| step.label == "process_checkin"));
        assert_eq!(compress_flow_steps(&steps, true).len(), steps.len());
    }

    #[test]
    fn flow_subsystem_score_requires_seed_in_target_subsystem() {
        let registration_flow = FlowResult {
            query: "register".to_string(),
            seed: "finalize_registration".to_string(),
            steps: vec![
                FlowStep {
                    label: "finalize_registration".to_string(),
                    kind: "function".to_string(),
                    file: "registration/app.py".to_string(),
                    line: Some(1085),
                },
                FlowStep {
                    label: "get_face_embedding".to_string(),
                    kind: "function".to_string(),
                    file: "registration/app.py".to_string(),
                    line: Some(524),
                },
            ],
        };
        let gate_flow = FlowResult {
            query: "checkin".to_string(),
            seed: "checkin_verify_and_log".to_string(),
            steps: vec![
                FlowStep {
                    label: "checkin_verify_and_log".to_string(),
                    kind: "function".to_string(),
                    file: "gate/app.py".to_string(),
                    line: Some(964),
                },
                FlowStep {
                    label: "parse_qr_payload".to_string(),
                    kind: "function".to_string(),
                    file: "gate/qr_module.py".to_string(),
                    line: Some(159),
                },
            ],
        };

        assert!(flow_subsystem_score(&registration_flow, "registration") > 0);
        assert_eq!(flow_subsystem_score(&gate_flow, "registration"), 0);
        assert!(flow_subsystem_score(&gate_flow, "gate") > 0);
    }
}
