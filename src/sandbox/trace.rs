//! Sandbox violation tracing via macOS unified logging
//!
//! Streams sandbox denial logs from the system log in real-time,
//! filtering for relevant violations to help debug sandbox issues.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Handle to a running trace session
pub struct TraceSession {
    child: Child,
    running: Arc<AtomicBool>,
}

impl TraceSession {
    /// Start a new trace session that streams sandbox violations
    pub fn start() -> std::io::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Use macOS `log stream` to capture sandbox violations
        // Sandbox denials are logged by the kernel with sender "Sandbox"
        let mut child = Command::new("log")
            .args([
                "stream",
                "--predicate",
                // Capture sandbox denial messages from kernel
                r#"sender == "Sandbox" AND eventMessage CONTAINS "deny""#,
                "--style",
                "compact",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        // Spawn a thread to read and filter the log output
        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(line) = line {
                        // Filter for denial messages and format output
                        if let Some(formatted) = format_violation(&line) {
                            eprintln!("{}", formatted);
                        }
                    }
                }
            });
        }

        Ok(Self { child, running })
    }

    /// Stop the trace session
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for TraceSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Format a sandbox violation log line for display
fn format_violation(line: &str) -> Option<String> {
    // Skip header lines and empty lines
    if line.starts_with("Filtering") || line.starts_with("Timestamp") || line.trim().is_empty() {
        return None;
    }

    // Look for denial patterns in the log
    // Format: timestamp kernel: (Sandbox) Sandbox: process(pid) deny(1) operation target
    if !line.contains("deny") {
        return None;
    }

    // Extract the sandbox denial part
    // Look for "Sandbox: process(pid) deny(N) operation target"
    if let Some(sandbox_start) = line.find("Sandbox: ") {
        let sandbox_part = &line[sandbox_start + 9..]; // Skip "Sandbox: "

        // Parse: "process(pid) deny(N) operation target"
        let parts: Vec<&str> = sandbox_part.splitn(2, " deny").collect();
        if parts.len() < 2 {
            return None;
        }

        let process = parts[0].trim();
        let deny_rest = parts[1].trim();

        // Skip the "(N) " part to get "operation target"
        let op_target = if let Some(paren_end) = deny_rest.find(") ") {
            &deny_rest[paren_end + 2..]
        } else {
            deny_rest
        };

        // Split operation and target
        let op_parts: Vec<&str> = op_target.splitn(2, ' ').collect();
        let operation = op_parts.first().unwrap_or(&"unknown");
        let target = op_parts.get(1).unwrap_or(&"");

        // Categorize the violation
        let category = if operation.contains("network") {
            "\x1b[31m[NETWORK]\x1b[0m"
        } else if operation.contains("file-read") {
            "\x1b[33m[READ]\x1b[0m"
        } else if operation.contains("file-write") {
            "\x1b[35m[WRITE]\x1b[0m"
        } else if operation.contains("process") {
            "\x1b[36m[PROCESS]\x1b[0m"
        } else if operation.contains("mach") {
            "\x1b[34m[MACH]\x1b[0m"
        } else {
            "\x1b[90m[OTHER]\x1b[0m"
        };

        return Some(format!(
            "\x1b[90m[sx:trace]\x1b[0m {} \x1b[1m{}\x1b[0m {} \x1b[90m({})\x1b[0m",
            category, operation, target, process
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_violation_network() {
        // Real kernel log format
        let line = r#"2024-01-01 12:00:00.000 kernel: (Sandbox) Sandbox: curl(1234) deny(1) network-outbound /private/var/run/mDNSResponder"#;
        let result = format_violation(line);
        assert!(result.is_some());
        let formatted = result.unwrap();
        assert!(formatted.contains("[NETWORK]"));
        assert!(formatted.contains("network-outbound"));
        assert!(formatted.contains("curl(1234)"));
    }

    #[test]
    fn test_format_violation_file_read() {
        let line = r#"2024-01-01 12:00:00.000 kernel: (Sandbox) Sandbox: node(5678) deny(1) file-read-data /Users/test/.ssh/id_rsa"#;
        let result = format_violation(line);
        assert!(result.is_some());
        let formatted = result.unwrap();
        assert!(formatted.contains("[READ]"));
        assert!(formatted.contains("file-read-data"));
    }

    #[test]
    fn test_format_violation_file_write() {
        let line = r#"2024-01-01 12:00:00.000 kernel: (Sandbox) Sandbox: node(5678) deny(1) file-write-data /etc/passwd"#;
        let result = format_violation(line);
        assert!(result.is_some());
        let formatted = result.unwrap();
        assert!(formatted.contains("[WRITE]"));
    }

    #[test]
    fn test_format_violation_skips_header() {
        let line = "Filtering the log data using...";
        assert!(format_violation(line).is_none());
    }

    #[test]
    fn test_format_violation_skips_non_deny() {
        let line = "2024-01-01 12:00:00.000 kernel: (Sandbox) some other message";
        assert!(format_violation(line).is_none());
    }
}
