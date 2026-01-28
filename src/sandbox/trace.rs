//! Sandbox violation tracing via macOS unified logging
//!
//! Streams sandbox denial logs from the system log in real-time,
//! filtering for relevant violations to help debug sandbox issues.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Category of sandbox violation for type-safe handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    Network,
    Read,
    Write,
    Process,
    Mach,
    Other,
}

impl ViolationKind {
    /// Get ANSI-colored display string for this violation kind
    pub fn colored(&self) -> &'static str {
        match self {
            ViolationKind::Network => "\x1b[31m[NETWORK]\x1b[0m",
            ViolationKind::Read => "\x1b[33m[READ]\x1b[0m",
            ViolationKind::Write => "\x1b[35m[WRITE]\x1b[0m",
            ViolationKind::Process => "\x1b[36m[PROCESS]\x1b[0m",
            ViolationKind::Mach => "\x1b[34m[MACH]\x1b[0m",
            ViolationKind::Other => "\x1b[90m[OTHER]\x1b[0m",
        }
    }

    /// Get plain text display string for this violation kind
    pub fn plain(&self) -> &'static str {
        match self {
            ViolationKind::Network => "[NETWORK]",
            ViolationKind::Read => "[READ]",
            ViolationKind::Write => "[WRITE]",
            ViolationKind::Process => "[PROCESS]",
            ViolationKind::Mach => "[MACH]",
            ViolationKind::Other => "[OTHER]",
        }
    }

    /// Determine violation kind from operation string
    fn from_operation(operation: &str) -> Self {
        if operation.contains("network") {
            ViolationKind::Network
        } else if operation.contains("file-read") {
            ViolationKind::Read
        } else if operation.contains("file-write") {
            ViolationKind::Write
        } else if operation.contains("process") {
            ViolationKind::Process
        } else if operation.contains("mach") {
            ViolationKind::Mach
        } else {
            ViolationKind::Other
        }
    }
}

/// Destination for trace output
enum TraceOutput {
    Stderr,
    File(File),
}

/// Handle to a running trace session
pub struct TraceSession {
    child: Child,
    running: Arc<AtomicBool>,
}

impl TraceSession {
    /// Start a new trace session that streams sandbox violations to stderr
    pub fn start() -> std::io::Result<Self> {
        Self::start_with_output(TraceOutput::Stderr)
    }

    /// Start a new trace session that streams sandbox violations to a file
    pub fn start_to_file(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Self::start_with_output(TraceOutput::File(file))
    }

    /// Start a trace session with specified output destination
    fn start_with_output(output: TraceOutput) -> std::io::Result<Self> {
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
        // Move ownership of output directly into the thread (no Arc<Mutex> needed)
        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                let mut output = output;
                let mut write_error_logged = false;

                for line in reader.lines() {
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(line) = line {
                        // Filter for denial messages and format output
                        if let Some(formatted) = format_violation(&line) {
                            match &mut output {
                                TraceOutput::File(file) => {
                                    // Write to file (strip ANSI codes for file output)
                                    let plain = strip_ansi_codes(&formatted);
                                    if let Err(e) = writeln!(file, "{}", plain) {
                                        // Log error once to avoid spam
                                        if !write_error_logged {
                                            eprintln!(
                                                "\x1b[33m[sx:trace]\x1b[0m Warning: failed to write to trace file: {}",
                                                e
                                            );
                                            write_error_logged = true;
                                        }
                                    }
                                    // Best effort flush, don't spam errors for this
                                    let _ = file.flush();
                                }
                                TraceOutput::Stderr => {
                                    eprintln!("{}", formatted);
                                }
                            }
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

        // Kill the log stream process
        if let Err(e) = self.child.kill() {
            // ESRCH (no such process) is expected if already exited
            if e.kind() != ErrorKind::NotFound && e.kind() != ErrorKind::InvalidInput {
                eprintln!(
                    "\x1b[33m[sx:trace]\x1b[0m Warning: failed to stop log stream: {}",
                    e
                );
            }
        }

        // Always try to reap the child process to prevent zombies
        if let Err(e) = self.child.wait() {
            eprintln!(
                "\x1b[33m[sx:trace]\x1b[0m Warning: failed to wait for log stream: {}",
                e
            );
        }
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
    let sandbox_start = line.find("Sandbox: ")?;
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

    // Categorize the violation using type-safe enum
    let kind = ViolationKind::from_operation(operation);

    Some(format!(
        "\x1b[90m[sx:trace]\x1b[0m {} \x1b[1m{}\x1b[0m {} \x1b[90m({})\x1b[0m",
        kind.colored(),
        operation,
        target,
        process
    ))
}

/// Strip ANSI escape codes from a string for plain text output
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we hit a letter (end of ANSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ViolationKind Tests ===

    #[test]
    fn test_violation_kind_from_operation() {
        assert_eq!(
            ViolationKind::from_operation("network-outbound"),
            ViolationKind::Network
        );
        assert_eq!(
            ViolationKind::from_operation("file-read-data"),
            ViolationKind::Read
        );
        assert_eq!(
            ViolationKind::from_operation("file-write-data"),
            ViolationKind::Write
        );
        assert_eq!(
            ViolationKind::from_operation("process-exec"),
            ViolationKind::Process
        );
        assert_eq!(
            ViolationKind::from_operation("mach-lookup"),
            ViolationKind::Mach
        );
        assert_eq!(
            ViolationKind::from_operation("unknown-op"),
            ViolationKind::Other
        );
    }

    #[test]
    fn test_violation_kind_colored() {
        assert!(ViolationKind::Network.colored().contains("31m")); // Red
        assert!(ViolationKind::Read.colored().contains("33m")); // Yellow
        assert!(ViolationKind::Write.colored().contains("35m")); // Magenta
    }

    #[test]
    fn test_violation_kind_plain() {
        assert_eq!(ViolationKind::Network.plain(), "[NETWORK]");
        assert_eq!(ViolationKind::Read.plain(), "[READ]");
        assert_eq!(ViolationKind::Write.plain(), "[WRITE]");
    }

    // === Format Violation Tests ===

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

    #[test]
    fn test_strip_ansi_codes() {
        let input = "\x1b[90m[sx:trace]\x1b[0m \x1b[31m[NETWORK]\x1b[0m \x1b[1mnetwork-outbound\x1b[0m target";
        let result = strip_ansi_codes(input);
        assert_eq!(result, "[sx:trace] [NETWORK] network-outbound target");
    }

    #[test]
    fn test_strip_ansi_codes_no_codes() {
        let input = "plain text without codes";
        let result = strip_ansi_codes(input);
        assert_eq!(result, input);
    }
}
