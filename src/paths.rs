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
        || normalized.starts_with("test/")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("/__tests__/")
        || normalized.starts_with("__tests__/")
        || normalized.ends_with("/conftest.py")
    {
        return true;
    }

    normalized.rsplit('/').next().is_some_and(|name| {
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || name.ends_with(".test.js")
            || name.ends_with(".test.ts")
            || name.ends_with(".spec.js")
            || name.ends_with(".spec.ts")
    })
}

pub fn is_project_metadata_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    if normalized.starts_with(".github/")
        || normalized.starts_with("scripts/")
        || normalized.starts_with("docs/")
        || normalized.starts_with("examples/")
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
                | ".editorconfig"
                | ".eslintignore"
                | ".gitattributes"
                | ".npmrc"
                | ".npmignore"
                | ".prettierignore"
                | "license"
                | "license.md"
                | "makefile"
        )
    )
}

pub fn is_excluded_from_clustering(path: &str) -> bool {
    is_documentation_file(path) || is_test_path(path) || is_project_metadata_path(path)
}

/// Docs, config, and deployment artifacts — hidden from `top-files` by default.
pub fn is_config_or_docs_path(path: &str) -> bool {
    if is_documentation_file(path) || is_project_metadata_path(path) {
        return true;
    }

    let normalized = normalize_path(path);
    let name = normalized.rsplit('/').next().unwrap_or("");

    if name.starts_with(".env") {
        return true;
    }

    if matches!(
        name,
        ".gitignore"
            | ".dockerignore"
            | ".cursorignore"
            | "requirements.txt"
            | "requirements-dev.txt"
            | "pyproject.toml"
            | "package.json"
            | "package-lock.json"
            | "cargo.toml"
            | "cargo.lock"
            | "go.mod"
            | "go.sum"
            | "procfile"
            | "uv.lock"
            | "poetry.lock"
            | "wrangler.toml"
            | "dockerfile"
            | "makefile"
            | "license"
            | "license.md"
    ) {
        return true;
    }

    name.ends_with(".sh")
        || name.ends_with(".toml")
        || name.ends_with(".lock")
        || (name.ends_with(".txt") && !name.ends_with(".c") && !name.ends_with(".h"))
        || (name.ends_with(".yml") || name.ends_with(".yaml"))
        || name.ends_with(".html")
        || name.ends_with(".json")
}

pub fn is_test_subsystem_key(key: &str) -> bool {
    let normalized = normalize_path(key);
    normalized == "tests"
        || normalized.starts_with("tests/")
        || normalized == "test"
        || normalized.starts_with("test/")
        || normalized == "__tests__"
        || normalized.starts_with("__tests__/")
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
        assert!(is_test_path("test/app.router.js"));
        assert!(is_test_path("tests/middleware/test_cors.py"));
        assert!(is_test_path("src/routes/user.spec.ts"));
        assert!(is_test_path("src/__tests__/router.js"));
        assert!(!is_test_path("starlette/routing.py"));
    }

    #[test]
    fn detects_metadata_paths() {
        assert!(is_project_metadata_path(".github/workflows/main.yml"));
        assert!(is_project_metadata_path(".editorconfig"));
        assert!(is_project_metadata_path("examples/auth/index.js"));
        assert!(is_project_metadata_path("mkdocs.yml"));
        assert!(!is_project_metadata_path("starlette/routing.py"));
    }

    #[test]
    fn detects_config_and_docs_for_top_files() {
        assert!(is_config_or_docs_path("README.md"));
        assert!(is_config_or_docs_path(".env.example"));
        assert!(is_config_or_docs_path("gate/requirements.txt"));
        assert!(is_config_or_docs_path("admin/run_dashboard.sh"));
        assert!(!is_config_or_docs_path("registration/app.py"));
        assert!(!is_config_or_docs_path("gate/qr_module.py"));
    }
}
