use std::path::Path;

use crate::graph::RankedFile;
use crate::intelligence::architecture::{self, subsystem_key};
use crate::paths;
#[derive(Debug, Clone)]
pub struct LearnStep {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct LearnResult {
    pub topic: String,
    pub subsystem: String,
    pub steps: Vec<LearnStep>,
    pub estimated_minutes: usize,
}

pub fn find_subsystem<'a>(
    subsystems: &'a [architecture::Subsystem],
    topic: &str,
) -> Result<&'a architecture::Subsystem, String> {
    let normalized_topic = topic.trim().to_lowercase();
    if normalized_topic.is_empty() {
        return Err("topic cannot be empty".to_string());
    }

    subsystems
        .iter()
        .filter(|subsystem| {
            subsystem.name.to_lowercase().contains(&normalized_topic)
                || subsystem.key.to_lowercase().contains(&normalized_topic)
        })
        .max_by_key(|subsystem| subsystem_priority(subsystem, &normalized_topic))
        .ok_or_else(|| {
            format!("no subsystem matched '{topic}' — try `atlas architecture` to see names")
        })
}

pub fn build_learning_path(repo: &Path, topic: &str) -> Result<LearnResult, String> {
    let report = architecture::analyze(repo)?;
    let ranked = crate::graph::top_files(repo, usize::MAX)?;

    let subsystem = find_subsystem(&report.subsystems, topic)?;

    let mut candidates: Vec<&RankedFile> = ranked
        .iter()
        .filter(|file| !paths::is_excluded_from_clustering(&file.file_path))
        .filter(|file| subsystem_key(&file.file_path) == subsystem.key)
        .collect();
    if candidates.is_empty() {
        return Err(format!(
            "subsystem '{}' matched but no ranked files were found — rescan the repository",
            subsystem.name
        ));
    }

    candidates.sort_by(|left, right| {
        learn_priority(&left.file_path)
            .cmp(&learn_priority(&right.file_path))
            .then_with(|| {
                right
                    .is_entrypoint
                    .cmp(&left.is_entrypoint)
            })
            .then_with(|| {
                right
                    .score
                    .partial_cmp(&left.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.file_path.cmp(&right.file_path))
    });

    let steps = candidates
        .into_iter()
        .map(|file| LearnStep {
            path: file.file_path.clone(),
            reason: if file.file_path.to_lowercase().contains("routes") {
                "HTTP routes and handlers".to_string()
            } else if file.is_entrypoint {
                "entrypoint".to_string()
            } else if file.file_path.to_lowercase().contains("service") {
                "business logic layer".to_string()
            } else if file.file_path.to_lowercase().contains("repositor") {
                "data access layer".to_string()
            } else if file.inbound_refs > 0 {
                format!("depended on by {} file(s)", file.inbound_refs)
            } else {
                "core file in subsystem".to_string()
            },
        })
        .collect::<Vec<_>>();

    let estimated_minutes = steps.len() * 5;

    Ok(LearnResult {
        topic: topic.to_string(),
        subsystem: subsystem.name.clone(),
        steps,
        estimated_minutes,
    })
}

fn subsystem_priority(subsystem: &architecture::Subsystem, topic: &str) -> i32 {
    let key = subsystem.key.to_lowercase();
    let name = subsystem.name.to_lowercase();
    let mut score = subsystem.total_score as i32;

    if paths::is_test_subsystem_key(&subsystem.key) {
        score -= 10_000;
    }

    if key == topic || key.ends_with(&format!("/{topic}")) {
        score += 500;
    }

    if name == topic {
        score += 300;
    }

    score
}

fn learn_priority(path: &str) -> u8 {
    let normalized = path.replace('\\', "/").to_lowercase();
    if normalized.contains("routes") {
        return 0;
    }
    if normalized.ends_with("main.py") || normalized.ends_with("main.go") {
        return 1;
    }
    if normalized.contains("service") {
        return 2;
    }
    if normalized.contains("repositor") || normalized.contains("/models") {
        return 3;
    }
    if normalized.ends_with("__init__.py") {
        return 5;
    }
    4
}
