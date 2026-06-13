pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

pub fn is_documentation_file(path: &str) -> bool {
    let normalized = normalize_path(path);
    normalized.ends_with(".md")
        || normalized.ends_with(".txt")
        || normalized.ends_with(".rst")
        || normalized.contains("/docs/")
}

pub fn is_test_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    if normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.ends_with("/conftest.py")
    {
        return true;
    }

    normalized
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("test_"))
}

pub fn is_project_metadata_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    if normalized.starts_with(".github/")
        || normalized.starts_with("scripts/")
        || normalized.starts_with("docs/")
        || normalized.contains("/docs/overrides/")
    {
        return true;
    }

    matches!(
        normalized.rsplit('/').next(),
        Some(
            ".gitignore"
                | "mkdocs.yml"
                | "citation.cff"
                | "codecov.yml"
                | "license"
                | "license.md"
                | "makefile"
        )
    )
}

pub fn is_excluded_from_clustering(path: &str) -> bool {
    is_documentation_file(path) || is_test_path(path) || is_project_metadata_path(path)
}

pub fn is_test_subsystem_key(key: &str) -> bool {
    let normalized = normalize_path(key);
    normalized == "tests" || normalized.starts_with("tests/")
}

/// Dead-code and migration leftovers — deprioritize in ranking and call resolution.
pub fn is_deprioritized_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    normalized.contains("/legacy")
        || normalized.starts_with("legacy/")
        || normalized.contains("/temp/")
        || normalized.starts_with("temp/")
        || normalized.contains("unused")
        || normalized.contains("_backup")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_test_paths() {
        assert!(is_test_path("tests/test_routing.py"));
        assert!(is_test_path("tests/middleware/test_cors.py"));
        assert!(!is_test_path("starlette/routing.py"));
    }

    #[test]
    fn detects_metadata_paths() {
        assert!(is_project_metadata_path(".github/workflows/main.yml"));
        assert!(is_project_metadata_path("mkdocs.yml"));
        assert!(!is_project_metadata_path("starlette/routing.py"));
    }
}
