// Violation logging/parsing
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A sandbox violation entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub timestamp: String,
    pub pid: Option<u32>,
    pub operation: String,
    pub path: Option<String>,
    pub process: Option<String>,
}

impl Violation {
    /// Parse a violation from a log line
    pub fn parse(line: &str) -> Option<Self> {
        // Example macOS sandbox violation log format:
        // sandboxd: deny(1) file-read-data /Users/test/.ssh/id_rsa
        // sandboxd: ([pid]/[process]): deny [operation] [path]

        if !line.contains("sandboxd") && !line.contains("deny") {
            return None;
        }

        let mut violation = Violation {
            timestamp: String::new(),
            pid: None,
            operation: String::new(),
            path: None,
            process: None,
        };

        // Extract operation and path
        if let Some(deny_idx) = line.find("deny") {
            let after_deny = &line[deny_idx..];
            let parts: Vec<&str> = after_deny.split_whitespace().collect();

            if parts.len() >= 2 {
                // Operation is usually after "deny" or "deny(N)"
                let op = parts.get(1).unwrap_or(&"");
                violation.operation = op.to_string();

                // Path is usually the last part
                if parts.len() >= 3 {
                    violation.path = Some(parts[2..].join(" "));
                }
            }
        }

        // Extract PID if present
        if let Some(pid_start) = line.find('(') {
            if let Some(pid_end) = line[pid_start..].find(')') {
                let pid_str = &line[pid_start + 1..pid_start + pid_end];
                violation.pid = pid_str.parse().ok();
            }
        }

        // Only return if we found meaningful data
        if !violation.operation.is_empty() || violation.path.is_some() {
            Some(violation)
        } else {
            None
        }
    }

    /// Format as a log line
    pub fn to_log_line(&self) -> String {
        let mut line = String::new();

        if !self.timestamp.is_empty() {
            line.push_str(&self.timestamp);
            line.push(' ');
        }

        line.push_str("deny ");
        line.push_str(&self.operation);

        if let Some(path) = &self.path {
            line.push(' ');
            line.push_str(path);
        }

        line
    }
}

/// Log a violation to a file
pub fn log_violation(log_path: &Path, violation: &Violation) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    writeln!(file, "{}", violation.to_log_line())?;
    Ok(())
}

/// Read violations from a log file
pub fn read_violations(log_path: &Path) -> io::Result<Vec<Violation>> {
    let file = File::open(log_path)?;
    let reader = BufReader::new(file);

    let violations: Vec<Violation> = reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| Violation::parse(&line))
        .collect();

    Ok(violations)
}

/// Get the default log file path
pub fn default_log_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("sx/violations.log"))
}

/// Ensure the log directory exists
pub fn ensure_log_dir(log_path: &Path) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_violation_deny_line() {
        let line = "sandboxd: deny file-read-data /Users/test/.ssh/id_rsa";
        let violation = Violation::parse(line).unwrap();
        assert_eq!(violation.operation, "file-read-data");
        assert_eq!(violation.path, Some("/Users/test/.ssh/id_rsa".to_string()));
    }

    #[test]
    fn test_parse_violation_with_pid() {
        let line = "sandboxd: (1234) deny file-read-data /path/to/file";
        let violation = Violation::parse(line).unwrap();
        assert_eq!(violation.pid, Some(1234));
    }

    #[test]
    fn test_parse_non_violation_returns_none() {
        let line = "some random log line";
        assert!(Violation::parse(line).is_none());
    }

    #[test]
    fn test_to_log_line() {
        let violation = Violation {
            timestamp: "2024-01-01".to_string(),
            pid: Some(1234),
            operation: "file-read-data".to_string(),
            path: Some("/Users/test/.ssh/id_rsa".to_string()),
            process: None,
        };

        let log_line = violation.to_log_line();
        assert!(log_line.contains("deny"));
        assert!(log_line.contains("file-read-data"));
        assert!(log_line.contains("/Users/test/.ssh/id_rsa"));
    }

    #[test]
    fn test_log_and_read_violations() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("violations.log");

        let violation = Violation {
            timestamp: String::new(),
            pid: None,
            operation: "file-read-data".to_string(),
            path: Some("/Users/test/.ssh/id_rsa".to_string()),
            process: None,
        };

        log_violation(&log_path, &violation).unwrap();

        let violations = read_violations(&log_path).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].operation, "file-read-data");
    }

    #[test]
    fn test_default_log_path() {
        let path = default_log_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("sx"));
        assert!(p.to_string_lossy().contains("violations.log"));
    }
}
