use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::graph::RankedFile;
use crate::intelligence::architecture::{self, subsystem_key, ArchitectureReport, Subsystem};
use crate::intelligence::flow;
use crate::intelligence::learn;
use crate::intelligence::snippets::{self, FileSnippet};
use crate::paths;

const MAX_CITATIONS: usize = 8;
const MAX_SNIPPETS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplainMatchKind {
    Subsystem,
    Flow,
    Path,
}

#[derive(Debug, Clone)]
pub struct ExplainCitation {
    pub path: String,
    pub role: String,
    pub score: f64,
    pub inbound_refs: usize,
    pub anchor_line: Option<usize>,
    pub anchor_symbol: Option<String>,
    pub snippet: Option<FileSnippet>,
}

#[derive(Debug, Clone)]
pub struct ExplainEvidence {
    pub topic: String,
    pub repository_name: String,
    pub match_kind: ExplainMatchKind,
    pub flow_seed: Option<String>,
    pub subsystem_name: String,
    pub subsystem_key: String,
    pub file_count: usize,
    pub internal_links: usize,
    pub citations: Vec<ExplainCitation>,
    pub entrypoints: Vec<String>,
    pub wiring_hints: Vec<String>,
    pub execution_flow: Vec<String>,
    pub purpose: Vec<String>,
    pub request_walkthrough: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OverviewReadingStep {
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct ExplainOverview {
    pub summary_lines: Vec<String>,
    pub reading_steps: Vec<OverviewReadingStep>,
}

pub fn gather_evidence(repo: &Path, topic: &str) -> Result<ExplainEvidence, String> {
    let normalized_topic = topic.trim().to_lowercase();
    if normalized_topic.is_empty() {
        return Err("explain topic cannot be empty".to_string());
    }

    let report = architecture::analyze(repo)?;

    let mut evidence = if let Ok(subsystem) = learn::find_subsystem(&report.subsystems, topic) {
        gather_from_subsystem(repo, topic, &report, subsystem)?
    } else if let Ok(flow) = flow::extract_flow(repo, topic) {
        gather_from_flow(repo, topic, &report, &flow)?
    } else {
        gather_from_path_match(repo, topic, &report)?
    };

    attach_snippets(repo, topic, &mut evidence)?;
    enrich_context(repo, &mut evidence)?;
    Ok(evidence)
}

fn enrich_context(repo: &Path, evidence: &mut ExplainEvidence) -> Result<(), String> {
    evidence.wiring_hints = build_wiring_hints(repo, evidence)?;
    evidence.execution_flow = build_execution_flow(repo, evidence)?;
    evidence.purpose = build_purpose(repo, evidence)?;
    evidence.request_walkthrough = build_request_walkthrough(repo, evidence)?;
    Ok(())
}

fn gather_from_subsystem(
    repo: &Path,
    topic: &str,
    report: &ArchitectureReport,
    subsystem: &Subsystem,
) -> Result<ExplainEvidence, String> {
    let plan = learn::build_learning_path(repo, topic)?;
    let ranked = crate::graph::top_files(repo, usize::MAX)?;
    let citations = citations_from_learn_steps(&plan.steps, &ranked);

    Ok(ExplainEvidence {
        topic: topic.to_string(),
        repository_name: report.repository_name.clone(),
        match_kind: ExplainMatchKind::Subsystem,
        flow_seed: None,
        subsystem_name: subsystem.name.clone(),
        subsystem_key: subsystem.key.clone(),
        file_count: subsystem.file_count,
        internal_links: subsystem.internal_links,
        citations,
        entrypoints: entrypoints_for_key(report, &subsystem.key),
        wiring_hints: Vec::new(),
        execution_flow: Vec::new(),
        purpose: Vec::new(),
        request_walkthrough: Vec::new(),
    })
}

fn gather_from_flow(
    repo: &Path,
    topic: &str,
    report: &ArchitectureReport,
    flow: &flow::FlowResult,
) -> Result<ExplainEvidence, String> {
    let ranked = crate::graph::top_files(repo, usize::MAX)?;
    let ranked_by_path: HashMap<String, &RankedFile> = ranked
        .iter()
        .map(|file| (file.file_path.clone(), file))
        .collect();

    let mut seen = HashSet::new();
    let mut citations = Vec::new();

    for step in &flow.steps {
        if !seen.insert(step.file.clone()) {
            continue;
        }
        let file = ranked_by_path.get(&step.file);
        citations.push(ExplainCitation {
            path: step.file.clone(),
            role: format!("flow step: {}", step.label),
            score: file.map(|entry| entry.score).unwrap_or(0.0),
            inbound_refs: file.map(|entry| entry.inbound_refs).unwrap_or(0),
            anchor_line: step.line,
            anchor_symbol: Some(step.label.clone()),
            snippet: None,
        });
        if citations.len() >= MAX_CITATIONS {
            break;
        }
    }

    let dominant_key = citations
        .first()
        .map(|citation| subsystem_key(&citation.path))
        .unwrap_or_else(|| "(root)".to_string());
    let subsystem = report
        .subsystems
        .iter()
        .find(|subsystem| subsystem.key == dominant_key);

    Ok(ExplainEvidence {
        topic: topic.to_string(),
        repository_name: report.repository_name.clone(),
        match_kind: ExplainMatchKind::Flow,
        flow_seed: Some(flow.seed.clone()),
        subsystem_name: subsystem
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| "Flow trace".to_string()),
        subsystem_key: dominant_key.clone(),
        file_count: citations.len(),
        internal_links: subsystem.map(|entry| entry.internal_links).unwrap_or(0),
        citations,
        entrypoints: entrypoints_for_key(report, &dominant_key),
        wiring_hints: Vec::new(),
        execution_flow: Vec::new(),
        purpose: Vec::new(),
        request_walkthrough: Vec::new(),
    })
}

fn gather_from_path_match(
    repo: &Path,
    topic: &str,
    report: &ArchitectureReport,
) -> Result<ExplainEvidence, String> {
    let normalized_topic = topic.trim().to_lowercase();
    let ranked = crate::graph::top_files(repo, usize::MAX)?;
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;

    let mut matched_paths = HashSet::new();
    for file in &ranked {
        if paths::is_excluded_from_clustering(&file.file_path) {
            continue;
        }
        if file.file_path.to_lowercase().contains(&normalized_topic) {
            matched_paths.insert(file.file_path.clone());
        }
    }

    for parsed in &symbols.files {
        if paths::is_excluded_from_clustering(&parsed.path) {
            continue;
        }
        let name_match = parsed
            .definitions
            .iter()
            .any(|definition| definition.name.to_lowercase().contains(&normalized_topic));
        if name_match {
            matched_paths.insert(parsed.path.clone());
        }
    }

    if matched_paths.is_empty() {
        return Err(format!(
            "no subsystem, flow, or file match for '{topic}' — try `atlas architecture` or `atlas flow {topic}`"
        ));
    }

    let mut matches: Vec<&RankedFile> = ranked
        .iter()
        .filter(|file| matched_paths.contains(&file.file_path))
        .collect();
    matches.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.file_path.cmp(&right.file_path))
    });

    let citations = matches
        .into_iter()
        .take(MAX_CITATIONS)
        .map(|file| ExplainCitation {
            path: file.file_path.clone(),
            role: "path or symbol name match".to_string(),
            score: file.score,
            inbound_refs: file.inbound_refs,
            anchor_line: None,
            anchor_symbol: None,
            snippet: None,
        })
        .collect::<Vec<_>>();

    let dominant_key = citations
        .first()
        .map(|citation| subsystem_key(&citation.path))
        .unwrap_or_else(|| "(root)".to_string());
    let subsystem = report
        .subsystems
        .iter()
        .find(|subsystem| subsystem.key == dominant_key);

    Ok(ExplainEvidence {
        topic: topic.to_string(),
        repository_name: report.repository_name.clone(),
        match_kind: ExplainMatchKind::Path,
        flow_seed: None,
        subsystem_name: subsystem
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| "Matched files".to_string()),
        subsystem_key: dominant_key.clone(),
        file_count: citations.len(),
        internal_links: subsystem.map(|entry| entry.internal_links).unwrap_or(0),
        citations,
        entrypoints: entrypoints_for_key(report, &dominant_key),
        wiring_hints: Vec::new(),
        execution_flow: Vec::new(),
        purpose: Vec::new(),
        request_walkthrough: Vec::new(),
    })
}

fn attach_snippets(repo: &Path, topic: &str, evidence: &mut ExplainEvidence) -> Result<(), String> {
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;
    let parsed_by_path: HashMap<String, &crate::parse::ParsedFile> = symbols
        .files
        .iter()
        .map(|file| (file.path.clone(), file))
        .collect();

    for citation in evidence.citations.iter_mut().take(MAX_SNIPPETS) {
        let Some(parsed) = parsed_by_path.get(&citation.path) else {
            continue;
        };

        let preferred = citation.anchor_symbol.as_deref();
        let Ok(snippet) = snippets::read_definition_snippet(
            repo,
            &citation.path,
            parsed,
            topic,
            &citation.role,
            preferred,
        ) else {
            continue;
        };

        if let Some((line, symbol)) =
            snippets::resolve_anchor(parsed, topic, &citation.role, preferred)
        {
            citation.anchor_line = Some(line);
            citation.anchor_symbol = Some(symbol);
        }

        citation.snippet = Some(snippet);
    }

    Ok(())
}

fn citations_from_learn_steps(
    steps: &[learn::LearnStep],
    ranked: &[RankedFile],
) -> Vec<ExplainCitation> {
    let ranked_by_path: HashMap<String, &RankedFile> = ranked
        .iter()
        .map(|file| (file.file_path.clone(), file))
        .collect();

    steps
        .iter()
        .take(MAX_CITATIONS)
        .map(|step| {
            let file = ranked_by_path.get(&step.path);
            ExplainCitation {
                path: step.path.clone(),
                role: step.reason.clone(),
                score: file.map(|entry| entry.score).unwrap_or(0.0),
                inbound_refs: file.map(|entry| entry.inbound_refs).unwrap_or(0),
                anchor_line: None,
                anchor_symbol: None,
                snippet: None,
            }
        })
        .collect()
}

fn entrypoints_for_key(report: &ArchitectureReport, key: &str) -> Vec<String> {
    report
        .entrypoints
        .iter()
        .filter(|entrypoint| {
            let entry_key = subsystem_key(entrypoint);
            entry_key == key || entry_key == "(root)"
        })
        .cloned()
        .collect()
}

fn subsystem_size_line(evidence: &ExplainEvidence) -> String {
    if evidence.internal_links == 0 {
        format!(
            "{} files · no direct imports within this folder",
            evidence.file_count
        )
    } else {
        format!(
            "{} files · {} direct import link(s) within this folder",
            evidence.file_count, evidence.internal_links
        )
    }
}

fn subsystem_wiring_note(evidence: &ExplainEvidence) -> Option<String> {
    if evidence.internal_links == 0 {
        Some(
            "Note: graph tracks import statements, not runtime wiring (e.g. middleware stacks)."
                .to_string(),
        )
    } else {
        None
    }
}

fn citation_reading_detail(citation: &ExplainCitation, index: usize) -> String {
    if index == 0 {
        format!(
            "{} · score {:.0} · {} inbound ref(s)",
            citation.role, citation.score, citation.inbound_refs
        )
    } else {
        format!("{} · score {:.0}", citation.role, citation.score)
    }
}

fn build_wiring_hints(repo: &Path, evidence: &ExplainEvidence) -> Result<Vec<String>, String> {
    let mut hints = Vec::new();
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;

    if evidence.entrypoints.is_empty() {
        hints.push(format!(
            "No entrypoint files inside {} — execution is wired from elsewhere.",
            evidence.subsystem_key
        ));
    }

    let wiring_symbols: &[&str] = if is_middleware_area(evidence) {
        &["add_middleware"]
    } else {
        &["add_middleware", "add_route", "mount", "include_router"]
    };

    for wiring_symbol in wiring_symbols {
        if let Some(wiring) = find_wiring_symbol(&symbols, wiring_symbol) {
            hints.push(format!(
                "Likely wired via {} ({}:{})",
                wiring.name, wiring.path, wiring.line
            ));
        }
    }

    if is_middleware_area(evidence) {
        hints.push(
            "Middleware classes are stacked at app setup; each __call__ delegates to the next with await self.app(scope, receive, send)."
                .to_string(),
        );
    }

    if hints.is_empty() && !evidence.entrypoints.is_empty() {
        hints.push("Entrypoint files in this subsystem start execution here.".to_string());
    }

    Ok(hints)
}

struct WiringSymbol {
    name: String,
    path: String,
    line: usize,
}

fn find_wiring_symbol(symbols: &crate::parse::ParseOutput, name: &str) -> Option<WiringSymbol> {
    symbols
        .files
        .iter()
        .flat_map(|parsed| {
            parsed.definitions.iter().filter_map(|definition| {
                if definition.name != name {
                    return None;
                }
                Some((
                    wiring_path_score(&parsed.path),
                    WiringSymbol {
                        name: definition.name.clone(),
                        path: parsed.path.clone(),
                        line: definition.line,
                    },
                ))
            })
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, wiring)| wiring)
}

fn wiring_path_score(path: &str) -> i32 {
    if paths::is_test_path(path) {
        return -1_000;
    }
    if path.replace('\\', "/").contains("applications.") {
        return 100;
    }
    if path.contains("main.") {
        return 90;
    }
    if path.contains("router") {
        return 50;
    }
    0
}

fn is_middleware_area(evidence: &ExplainEvidence) -> bool {
    evidence.subsystem_key.contains("middleware")
        || evidence.topic.to_lowercase().contains("middleware")
        || evidence
            .citations
            .iter()
            .any(|citation| citation.path.to_lowercase().contains("middleware"))
}

fn build_execution_flow(repo: &Path, evidence: &ExplainEvidence) -> Result<Vec<String>, String> {
    match evidence.match_kind {
        ExplainMatchKind::Flow => Ok(Vec::new()),
        ExplainMatchKind::Subsystem if is_middleware_area(evidence) => {
            middleware_execution_lines(repo, evidence)
        }
        ExplainMatchKind::Subsystem | ExplainMatchKind::Path => Ok(reading_order_flow(evidence)),
    }
}

fn build_request_walkthrough(
    repo: &Path,
    evidence: &ExplainEvidence,
) -> Result<Vec<String>, String> {
    if evidence.match_kind == ExplainMatchKind::Flow {
        return format_request_walkthrough(repo, evidence);
    }

    if evidence.match_kind != ExplainMatchKind::Subsystem || is_middleware_area(evidence) {
        return Ok(Vec::new());
    }

    let mut best: Option<(flow::FlowResult, i32)> = None;
    for query in walkthrough_queries(evidence) {
        let Ok(flow) = flow::extract_flow(repo, &query) else {
            continue;
        };
        if flow.steps.len() < 2 {
            continue;
        }
        if flow.seed.starts_with('_') || flow.steps[0].label.starts_with('_') {
            continue;
        }
        let score = walkthrough_flow_score(&flow, &evidence.subsystem_key);
        if score <= 0 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| score > *best_score)
        {
            best = Some((flow, score));
        }
    }

    let Some((flow, _)) = best else {
        return Ok(Vec::new());
    };

    format_subsystem_walkthrough(repo, &flow, &evidence.subsystem_key, false)
}

fn walkthrough_flow_score(flow: &flow::FlowResult, target_key: &str) -> i32 {
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
    let out_of_subsystem = flow.steps.len().saturating_sub(in_subsystem);
    if in_subsystem < 2 {
        return 0;
    }

    let mut score = flow::flow_subsystem_score(flow, target_key);
    score += in_subsystem as i32 * 20;
    score -= out_of_subsystem as i32 * 80;
    score
}

fn format_subsystem_walkthrough(
    repo: &Path,
    flow: &flow::FlowResult,
    target_key: &str,
    verbose: bool,
) -> Result<Vec<String>, String> {
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;
    let compressed = flow::compress_flow_steps(&flow.steps, verbose);
    let scoped: Vec<flow::FlowStep> = compressed
        .iter()
        .filter(|step| subsystem_key(&step.file) == target_key)
        .cloned()
        .collect();

    if scoped.len() < 2 {
        return Ok(Vec::new());
    }

    let scoped_flow = flow::FlowResult {
        query: flow.query.clone(),
        seed: flow.seed.clone(),
        steps: scoped,
    };
    format_flow_walkthrough_with_symbols(&scoped_flow, &symbols, verbose)
}

fn walkthrough_queries(evidence: &ExplainEvidence) -> Vec<String> {
    let topic_lower = evidence.topic.to_lowercase();
    let key_lower = evidence.subsystem_key.to_lowercase();
    let mut queries = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |value: &str| {
        if seen.insert(value.to_lowercase()) {
            queries.push(value.to_string());
        }
    };

    if topic_lower.contains("auth") || key_lower.contains("auth") {
        push("login");
        push("register");
        push("logout");
    }
    if topic_lower.contains("registration") || key_lower.contains("registration") {
        push("finalize_registration");
        push("verify_face");
        push("register");
        push("verify");
    }
    if topic_lower.contains("gate") || key_lower.contains("gate") {
        push("checkin");
        push("checkout");
    }
    if topic_lower.contains("order") || key_lower.contains("order") {
        push("order");
        push("create");
    }
    push(&evidence.topic);

    for citation in &evidence.citations {
        if !citation.path.to_lowercase().contains("routes") {
            continue;
        }
        if let Some(symbol) = &citation.anchor_symbol {
            push(symbol);
        }
    }

    queries
}

fn format_request_walkthrough(
    repo: &Path,
    evidence: &ExplainEvidence,
) -> Result<Vec<String>, String> {
    let query = evidence.flow_seed.as_deref().unwrap_or(&evidence.topic);

    let steps: Vec<flow::FlowStep> = evidence
        .citations
        .iter()
        .filter_map(|citation| {
            Some(flow::FlowStep {
                label: citation.anchor_symbol.clone()?,
                kind: String::new(),
                file: citation.path.clone(),
                line: citation.anchor_line,
            })
        })
        .collect();

    if steps.len() < 2 {
        return Ok(Vec::new());
    }

    let flow = flow::FlowResult {
        query: query.to_string(),
        seed: query.to_string(),
        steps,
    };
    format_flow_walkthrough(repo, &flow, false)
}

fn format_flow_walkthrough(
    repo: &Path,
    flow: &flow::FlowResult,
    verbose: bool,
) -> Result<Vec<String>, String> {
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;
    format_flow_walkthrough_with_symbols(flow, &symbols, verbose)
}

fn format_flow_walkthrough_with_symbols(
    flow: &flow::FlowResult,
    symbols: &crate::parse::ParseOutput,
    verbose: bool,
) -> Result<Vec<String>, String> {
    let display_steps = flow::compress_flow_steps(&flow.steps, verbose);
    let compressed = !verbose && display_steps.len() < flow.steps.len();

    let mut lines = vec![if compressed {
        "Call-graph walkthrough (compressed primary path — use `atlas flow --verbose` for full trace):"
                .to_string()
    } else {
        "Call-graph walkthrough (approximate — dynamic dispatch may be missing):".to_string()
    }];

    if let Some(label) = infer_request_label(flow, symbols) {
        lines.push(format!("  {label}"));
        lines.push("    ↓".to_string());
    }

    for (index, step) in display_steps.iter().enumerate() {
        if index > 0 {
            lines.push("    ↓".to_string());
        }
        lines.push(format!("  {}::{}", step.file, step.label));
    }

    lines.push("    ↓".to_string());
    lines.push("  response returned".to_string());
    Ok(lines)
}

fn infer_request_label(
    flow: &flow::FlowResult,
    symbols: &crate::parse::ParseOutput,
) -> Option<String> {
    let first = flow.steps.first()?;
    let parsed = symbols.files.iter().find(|file| file.path == first.file)?;

    if let Some(line) = first.line {
        if let Some(route) = parsed
            .definitions
            .iter()
            .filter(|definition| definition.kind == "route" && definition.line <= line)
            .max_by_key(|definition| definition.line)
        {
            if line.saturating_sub(route.line) <= 5 {
                return Some(format_http_route(&flow.query, &route.name));
            }
        }
    }

    parsed
        .definitions
        .iter()
        .find(|definition| {
            definition.kind == "route" && route_matches_query(&definition.name, &flow.query)
        })
        .map(|route| format_http_route(&flow.query, &route.name))
}

fn route_matches_query(route_path: &str, query: &str) -> bool {
    let route_lower = route_path.to_lowercase();
    let query_lower = query.to_lowercase();
    route_lower.contains(&query_lower) || query_lower.contains(route_lower.trim_start_matches('/'))
}

fn format_http_route(query: &str, route_path: &str) -> String {
    let path = normalize_route_path(route_path);
    let query_lower = query.to_lowercase();
    let method = if query_lower.contains("logout")
        || query_lower.contains("login")
        || query_lower.contains("register")
    {
        "POST"
    } else if query_lower.contains("get") {
        "GET"
    } else {
        "REQUEST"
    };
    format!("{method} {path}")
}

fn normalize_route_path(route_name: &str) -> String {
    if let Some(path) = route_name
        .split_whitespace()
        .find(|part| part.starts_with('/'))
    {
        return path.to_string();
    }
    if route_name.starts_with('/') {
        return route_name.to_string();
    }
    format!("/{route_name}")
}

fn middleware_execution_lines(
    repo: &Path,
    evidence: &ExplainEvidence,
) -> Result<Vec<String>, String> {
    let atlas_dir = crate::store::require_atlas_dir(repo)?;
    let symbols = crate::parse::load_symbols(&atlas_dir)?;
    let parsed_by_path: HashMap<String, &crate::parse::ParsedFile> = symbols
        .files
        .iter()
        .map(|file| (file.path.clone(), file))
        .collect();

    let mut lines = vec![
        "Possible request path — not the actual runtime stack (order depends on app configuration):".to_string(),
        "  Incoming request".to_string(),
        "    ↓".to_string(),
    ];

    if let Some(wiring) = find_wiring_symbol(&symbols, "add_middleware") {
        lines.push(format!(
            "  Middleware stack entry (configured via {}:{})",
            wiring.path, wiring.line
        ));
        lines.push("    ↓".to_string());
    }

    for citation in evidence.citations.iter().take(6) {
        let class_name = parsed_by_path
            .get(&citation.path)
            .and_then(|parsed| primary_class_name(parsed))
            .unwrap_or_else(|| file_stem_label(&citation.path));
        lines.push(format!("  {class_name} (representative layer)"));
        lines.push("    ↓".to_string());
    }

    lines.push("  Router → endpoint handler".to_string());
    lines.push("  Layers above are ranked by graph importance, not install order.".to_string());
    Ok(lines)
}

fn reading_order_flow(evidence: &ExplainEvidence) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, citation) in evidence.citations.iter().take(6).enumerate() {
        if index > 0 {
            lines.push("    ↓".to_string());
        }
        lines.push(format!("  {} ({})", citation.path, citation.role));
    }
    lines
}

fn primary_class_name(parsed: &crate::parse::ParsedFile) -> Option<String> {
    if let Some(definition) = parsed
        .definitions
        .iter()
        .find(|definition| definition.kind == "class" && definition.name.ends_with("Middleware"))
    {
        return Some(definition.name.clone());
    }

    parsed
        .definitions
        .iter()
        .find(|definition| definition.kind == "class")
        .map(|definition| definition.name.clone())
}

fn file_stem_label(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim_end_matches(".py")
        .trim_end_matches(".ts")
        .trim_end_matches(".go")
        .to_string()
}

pub fn execution_flow_heading(evidence: &ExplainEvidence) -> &str {
    if evidence.match_kind == ExplainMatchKind::Flow {
        "Request walkthrough"
    } else if is_middleware_area(evidence) {
        "Representative middleware flow"
    } else if evidence.match_kind == ExplainMatchKind::Subsystem {
        "Suggested reading flow"
    } else {
        "Execution flow"
    }
}

fn build_purpose(repo: &Path, evidence: &ExplainEvidence) -> Result<Vec<String>, String> {
    if is_middleware_area(evidence) {
        return Ok(middleware_purpose(repo, evidence));
    }

    let topic = evidence.topic.to_lowercase();
    let key = evidence.subsystem_key.to_lowercase();

    if topic.contains("auth") || key.contains("auth") {
        return Ok(vec![
            "Authentication verifies identity before protected handlers run.".to_string(),
            "Typical layers: HTTP routes/handlers, service logic, token or session storage, and user lookup.".to_string(),
        ]);
    }

    if topic.contains("route") || key.contains("route") || key.contains("api") {
        return Ok(vec![
            "Routing maps incoming HTTP paths and methods to handler functions.".to_string(),
            "Route modules declare endpoints; services and repositories implement the work behind them.".to_string(),
        ]);
    }

    if evidence.match_kind == ExplainMatchKind::Flow {
        let seed = evidence.flow_seed.as_deref().unwrap_or(&evidence.topic);
        return Ok(vec![
            format!(
                "This topic traces a call-graph path starting at \"{seed}\".",
            ),
            "Follow the citations below to see which functions call which — dynamic dispatch may be missing.".to_string(),
        ]);
    }

    Ok(vec![format!(
        "The {} area groups related code for \"{}\" — read ranked files below to see how responsibilities split across modules.",
        evidence.subsystem_name, evidence.topic
    )])
}

fn middleware_purpose(repo: &Path, evidence: &ExplainEvidence) -> Vec<String> {
    let mut paragraphs = vec![
        "Middleware intercepts requests before they reach route handlers.".to_string(),
        "Each layer wraps the next application component and can inspect or modify requests and responses.".to_string(),
    ];

    if let Ok(atlas_dir) = crate::store::require_atlas_dir(repo) {
        if let Ok(symbols) = crate::parse::load_symbols(&atlas_dir) {
            let responsibilities =
                detect_middleware_responsibilities(&evidence.citations, &symbols.files);
            if !responsibilities.is_empty() {
                paragraphs.push(format!(
                    "Components detected in this repo: {}.",
                    responsibilities.join(", ")
                ));
                return paragraphs;
            }
        }
    }

    paragraphs.push(
        "Common responsibilities in this pattern: sessions, error handling, compression, CORS, and authentication."
            .to_string(),
    );
    paragraphs
}

fn detect_middleware_responsibilities(
    citations: &[ExplainCitation],
    files: &[crate::parse::ParsedFile],
) -> Vec<String> {
    let parsed_by_path: HashMap<String, &crate::parse::ParsedFile> =
        files.iter().map(|file| (file.path.clone(), file)).collect();

    let mut responsibilities = Vec::new();

    for citation in citations {
        let path_lower = citation.path.to_lowercase();
        let class_name = parsed_by_path
            .get(&citation.path)
            .and_then(|parsed| primary_class_name(parsed))
            .unwrap_or_default()
            .to_lowercase();

        let label = if path_lower.contains("session") || class_name.contains("session") {
            Some("Sessions")
        } else if path_lower.contains("cors") || class_name.contains("cors") {
            Some("CORS")
        } else if path_lower.contains("gzip") || class_name.contains("gzip") {
            Some("Compression")
        } else if path_lower.contains("error") || class_name.contains("error") {
            Some("Error handling")
        } else if path_lower.contains("auth") || class_name.contains("auth") {
            Some("Authentication")
        } else if path_lower.contains("trusted") || class_name.contains("trusted") {
            Some("Host validation")
        } else if path_lower.contains("wsgi") || class_name.contains("wsgi") {
            Some("WSGI bridging")
        } else if path_lower.contains("exception") || class_name.contains("exception") {
            Some("Exception handling")
        } else if path_lower.contains("base") || class_name.contains("basehttp") {
            Some("Base dispatch wrapper")
        } else {
            None
        };

        if let Some(item) = label {
            if !responsibilities.iter().any(|existing| existing == item) {
                responsibilities.push(item.to_string());
            }
        }
    }

    responsibilities
}

pub fn build_overview(evidence: &ExplainEvidence) -> ExplainOverview {
    match evidence.match_kind {
        ExplainMatchKind::Subsystem => subsystem_overview(evidence),
        ExplainMatchKind::Flow => ExplainOverview {
            summary_lines: flow_overview(evidence),
            reading_steps: Vec::new(),
        },
        ExplainMatchKind::Path => ExplainOverview {
            summary_lines: path_overview(evidence),
            reading_steps: Vec::new(),
        },
    }
}

fn subsystem_overview(evidence: &ExplainEvidence) -> ExplainOverview {
    let mut summary_lines = vec![
        format!(
            "Matched \"{}\" → {} ({})",
            evidence.topic, evidence.subsystem_name, evidence.subsystem_key
        ),
        subsystem_size_line(evidence),
    ];

    if let Some(note) = subsystem_wiring_note(evidence) {
        summary_lines.push(note);
    }

    let reading_steps = evidence
        .citations
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, citation)| OverviewReadingStep {
            path: citation.path.clone(),
            detail: citation_reading_detail(citation, index),
        })
        .collect();

    ExplainOverview {
        summary_lines,
        reading_steps,
    }
}

fn flow_overview(evidence: &ExplainEvidence) -> Vec<String> {
    let seed = evidence.flow_seed.as_deref().unwrap_or(&evidence.topic);

    let mut paragraphs = vec![format!(
        "Atlas resolved \"{}\" as a call-graph flow starting at seed \"{}\" in the {} area.",
        evidence.topic, seed, evidence.subsystem_name
    )];

    if let Some(first) = evidence.citations.first() {
        let anchor = first
            .anchor_symbol
            .as_deref()
            .unwrap_or("the matched symbol");
        let downstream = evidence.citations.len().saturating_sub(1);
        if downstream == 0 {
            paragraphs.push(format!(
                "Atlas found the matching seed in {} at {}, but did not resolve a reliable downstream file transition.",
                first.path, anchor
            ));
        } else {
            paragraphs.push(format!(
                "The trace begins in {} at {} and continues through {} downstream file(s).",
                first.path, anchor, downstream
            ));
        }
    }

    let chain: Vec<String> = evidence
        .citations
        .iter()
        .filter_map(|citation| citation.anchor_symbol.clone())
        .collect();
    if chain.len() > 1 {
        paragraphs.push(format!("Call chain: {}.", chain.join(" → ")));
    }

    paragraphs
}

fn path_overview(evidence: &ExplainEvidence) -> Vec<String> {
    vec![
        format!(
            "Atlas matched \"{}\" by file path or symbol name in the {} area ({} hit(s) in the graph).",
            evidence.topic,
            evidence.subsystem_name,
            evidence.citations.len()
        ),
        format!(
            "Top matches: {}.",
            evidence
                .citations
                .iter()
                .take(4)
                .map(|citation| citation.path.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ]
}

pub fn all_cited_paths_exist_in_graph(
    repo: &Path,
    evidence: &ExplainEvidence,
) -> Result<bool, String> {
    let ranked = crate::graph::top_files_with_options(repo, usize::MAX, true, true)?;
    let known: std::collections::HashSet<&str> =
        ranked.iter().map(|file| file.file_path.as_str()).collect();

    Ok(evidence
        .citations
        .iter()
        .all(|citation| known.contains(citation.path.as_str())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_evidence() -> ExplainEvidence {
        ExplainEvidence {
            topic: "auth".to_string(),
            repository_name: "demo_app".to_string(),
            match_kind: ExplainMatchKind::Subsystem,
            flow_seed: None,
            subsystem_name: "Auth".to_string(),
            subsystem_key: "auth".to_string(),
            file_count: 5,
            internal_links: 4,
            citations: vec![
                ExplainCitation {
                    path: "auth/routes.py".to_string(),
                    role: "HTTP routes and handlers".to_string(),
                    score: 42.0,
                    inbound_refs: 2,
                    anchor_line: Some(21),
                    anchor_symbol: Some("login_handler".to_string()),
                    snippet: None,
                },
                ExplainCitation {
                    path: "auth/service.py".to_string(),
                    role: "business logic layer".to_string(),
                    score: 35.0,
                    inbound_refs: 1,
                    anchor_line: Some(16),
                    anchor_symbol: Some("login".to_string()),
                    snippet: None,
                },
            ],
            entrypoints: vec!["main.py".to_string()],
            wiring_hints: Vec::new(),
            execution_flow: Vec::new(),
            purpose: Vec::new(),
            request_walkthrough: Vec::new(),
        }
    }

    #[test]
    fn normalize_route_path_extracts_http_path() {
        assert_eq!(normalize_route_path("route.post /login"), "/login");
        assert_eq!(normalize_route_path("/logout"), "/logout");
    }

    #[test]
    fn auth_subsystem_includes_login_walkthrough() {
        let repo = Path::new("tests/fixtures/demo_app");
        if !repo.join(".atlas").is_dir() {
            return;
        }

        let evidence = gather_evidence(repo, "auth").expect("evidence");
        assert!(
            !evidence.request_walkthrough.is_empty(),
            "expected request walkthrough for auth subsystem"
        );
        let joined = evidence.request_walkthrough.join("\n");
        assert!(joined.contains("login_handler"));
        assert!(joined.contains("auth/service.py::login"));
        assert!(joined.contains("response returned"));
    }

    #[test]
    fn middleware_heading_is_non_authoritative() {
        let evidence = ExplainEvidence {
            topic: "middleware".to_string(),
            repository_name: "starlette".to_string(),
            match_kind: ExplainMatchKind::Subsystem,
            flow_seed: None,
            subsystem_name: "Middleware".to_string(),
            subsystem_key: "starlette/middleware".to_string(),
            file_count: 11,
            internal_links: 0,
            citations: vec![ExplainCitation {
                path: "starlette/middleware/base.py".to_string(),
                role: "core".to_string(),
                score: 42.0,
                inbound_refs: 3,
                anchor_line: None,
                anchor_symbol: None,
                snippet: None,
            }],
            entrypoints: Vec::new(),
            wiring_hints: Vec::new(),
            execution_flow: Vec::new(),
            purpose: Vec::new(),
            request_walkthrough: Vec::new(),
        };

        assert_eq!(
            execution_flow_heading(&evidence),
            "Representative middleware flow"
        );
    }

    #[test]
    fn auth_purpose_is_generated() {
        let evidence = sample_evidence();
        let purpose = build_purpose(Path::new("."), &evidence).expect("purpose");
        assert!(purpose.iter().any(|line| line.contains("Authentication")));
    }

    #[test]
    fn overview_has_multiple_paragraphs_for_subsystem() {
        let evidence = sample_evidence();
        let overview = build_overview(&evidence);
        assert!(overview.summary_lines[0].contains("Auth"));
        assert!(overview.reading_steps.len() >= 2);
        assert!(overview.reading_steps[0].path.contains("auth/routes.py"));
    }

    #[test]
    fn flow_overview_includes_call_chain() {
        let evidence = ExplainEvidence {
            match_kind: ExplainMatchKind::Flow,
            flow_seed: Some("login_handler".to_string()),
            citations: vec![
                ExplainCitation {
                    path: "auth/routes.py".to_string(),
                    role: "flow step: login_handler".to_string(),
                    score: 13.0,
                    inbound_refs: 1,
                    anchor_line: Some(21),
                    anchor_symbol: Some("login_handler".to_string()),
                    snippet: None,
                },
                ExplainCitation {
                    path: "auth/service.py".to_string(),
                    role: "flow step: login".to_string(),
                    score: 20.0,
                    inbound_refs: 2,
                    anchor_line: Some(16),
                    anchor_symbol: Some("login".to_string()),
                    snippet: None,
                },
            ],
            topic: "login".to_string(),
            repository_name: "demo_app".to_string(),
            subsystem_name: "Auth".to_string(),
            subsystem_key: "auth".to_string(),
            file_count: 2,
            internal_links: 3,
            entrypoints: vec!["main.py".to_string()],
            wiring_hints: Vec::new(),
            execution_flow: Vec::new(),
            purpose: Vec::new(),
            request_walkthrough: Vec::new(),
        };

        let paragraphs = flow_overview(&evidence);
        assert!(paragraphs.iter().any(|line| line.contains("login_handler")));
        assert!(paragraphs.iter().any(|line| line.contains("→")));
    }

    #[test]
    fn zero_internal_links_uses_precise_wording() {
        let mut evidence = sample_evidence();
        evidence.internal_links = 0;
        let overview = build_overview(&evidence);
        assert!(overview.summary_lines[1].contains("no direct imports within this folder"));
        assert!(overview
            .summary_lines
            .iter()
            .any(|line| line.contains("runtime wiring")));
    }

    #[test]
    fn flow_overview_handles_single_file_trace() {
        let evidence = ExplainEvidence {
            match_kind: ExplainMatchKind::Flow,
            flow_seed: Some("app".to_string()),
            citations: vec![ExplainCitation {
                path: "fastapi/routing.py".to_string(),
                role: "flow step: app".to_string(),
                score: 447.0,
                inbound_refs: 14,
                anchor_line: Some(119),
                anchor_symbol: Some("app".to_string()),
                snippet: None,
            }],
            topic: "routing".to_string(),
            repository_name: "fastapi".to_string(),
            subsystem_name: "Fastapi".to_string(),
            subsystem_key: "fastapi".to_string(),
            file_count: 1,
            internal_links: 0,
            entrypoints: Vec::new(),
            wiring_hints: Vec::new(),
            execution_flow: Vec::new(),
            purpose: Vec::new(),
            request_walkthrough: Vec::new(),
        };

        let paragraphs = flow_overview(&evidence);
        assert!(paragraphs
            .iter()
            .any(|line| line.contains("did not resolve a reliable downstream")));
        assert!(!paragraphs.iter().any(|line| line.contains("0 downstream")));
    }

    #[test]
    fn citation_paths_match_evidence() {
        let evidence = sample_evidence();
        let paths: Vec<&str> = evidence
            .citations
            .iter()
            .map(|citation| citation.path.as_str())
            .collect();
        assert_eq!(paths, vec!["auth/routes.py", "auth/service.py"]);
    }

    #[test]
    fn excluded_paths_are_not_clustered() {
        assert!(crate::paths::is_excluded_from_clustering(
            "tests/test_auth.py"
        ));
        assert!(!crate::paths::is_excluded_from_clustering(
            "auth/service.py"
        ));
    }
}
