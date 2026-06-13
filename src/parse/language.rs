use std::path::Path;

use tree_sitter::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageKind {
    Python,
    TypeScript,
    JavaScript,
    Go,
    C,
}

impl LanguageKind {
    pub fn from_path(path: &Path) -> Option<Self> {
        let file = path.file_name()?.to_str()?.to_lowercase();
        let ext = path.extension()?.to_str()?.to_lowercase();

        match ext.as_str() {
            "py" | "pyw" => Some(Self::Python),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::TypeScript),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "go" => Some(Self::Go),
            "c" | "h" => Some(Self::C),
            _ if file == "makefile" => None,
            _ => None,
        }
    }

    pub fn tree_sitter_language(self, path: &Path) -> Language {
        match self {
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::TypeScript => {
                if path.extension().and_then(|e| e.to_str()) == Some("tsx") {
                    tree_sitter_typescript::LANGUAGE_TSX.into()
                } else {
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
                }
            }
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::C => "c",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_c_and_header_files() {
        assert_eq!(LanguageKind::from_path(Path::new("kernel/sched.c")), Some(LanguageKind::C));
        assert_eq!(LanguageKind::from_path(Path::new("include/linux/mm.h")), Some(LanguageKind::C));
    }

    #[test]
    fn ignores_unsupported_extensions() {
        assert_eq!(LanguageKind::from_path(Path::new("README.md")), None);
    }
}
