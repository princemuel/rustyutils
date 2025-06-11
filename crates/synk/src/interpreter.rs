use ::std::path::Path;

/// Detects the appropriate interpreter for a script based on its file extension
pub fn detect_interpreter(path: &Path) -> Option<String> {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        match ext.to_lowercase().as_str() {
            "py" => Some("python3".to_string()),
            "js" => Some("node".to_string()),
            "rb" => Some("ruby".to_string()),
            "sh" => Some("bash".to_string()),
            "pl" => Some("perl".to_string()),
            "php" => Some("php".to_string()),
            "lua" => Some("lua".to_string()),
            "r" => Some("Rscript".to_string()),
            _ => None,
        }
    } else {
        None
    }
}

/// Gets a list of all supported interpreters
pub fn supported_interpreters() -> Vec<(&'static str, &'static str)> {
    vec![
        ("py", "python3"),
        ("js", "node"),
        ("rb", "ruby"),
        ("sh", "bash"),
        ("go", "go"),
        ("pl", "perl"),
        ("php", "php"),
        ("lua", "lua"),
        ("r", "Rscript"),
    ]
}

/// Checks if a file extension is supported
pub fn is_supported_extension(extension: &str) -> bool {
    supported_interpreters()
        .iter()
        .any(|(ext, _)| ext.eq_ignore_ascii_case(extension))
}

/// Gets the interpreter name for a given extension
pub fn get_interpreter_for_extension(extension: &str) -> Option<&'static str> {
    supported_interpreters()
        .iter()
        .find(|(ext, _)| ext.eq_ignore_ascii_case(extension))
        .map(|(_, interpreter)| *interpreter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_interpreter() {
        let test_cases = vec![
            ("test.py", Some("python3".to_string())),
            ("script.js", Some("node".to_string())),
            ("app.rb", Some("ruby".to_string())),
            ("script.sh", Some("bash".to_string())),
            ("program.pl", Some("perl".to_string())),
            ("unknown.xyz", None),
            ("no_extension", None),
        ];

        for (filename, expected) in test_cases {
            let path = PathBuf::from(filename);
            assert_eq!(detect_interpreter(&path), expected);
        }
    }

    #[test]
    fn test_is_supported_extension() {
        assert!(is_supported_extension("py"));
        assert!(is_supported_extension("PY"));
        assert!(is_supported_extension("js"));
        assert!(!is_supported_extension("xyz"));
    }

    #[test]
    fn test_get_interpreter_for_extension() {
        assert_eq!(get_interpreter_for_extension("py"), Some("python3"));
        assert_eq!(get_interpreter_for_extension("JS"), Some("node"));
        assert_eq!(get_interpreter_for_extension("xyz"), None);
    }
}
