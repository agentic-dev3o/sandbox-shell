//! Integration tests for signal forwarding (issue #37).
//!
//! Verifies that `sx` forwards SIGINT/SIGTERM/SIGHUP to the entire sandboxed
//! process subtree so descendants are not orphaned to launchd when `sx` exits.

use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Path to the `sx` binary produced by Cargo for this test target.
const SX_BIN: &str = env!("CARGO_BIN_EXE_sx");

/// Probe whether `sandbox-exec` accepts a custom deny-default profile on this
/// system. On hardened macOS configurations custom profiles can be blocked,
/// in which case there is nothing meaningful to assert about signal forwarding.
fn is_custom_sandbox_available() -> bool {
    let probe = r#"(version 1)
(deny default)
(allow process-fork)
(allow process-exec)
(allow signal (target self))
(allow sysctl-read)
(allow file-read-metadata)
(allow mach-lookup)
(allow file-read* (subpath "/usr"))
(allow file-read* (subpath "/bin"))
"#;
    let Ok(temp) = tempfile::NamedTempFile::new() else {
        return false;
    };
    if fs::write(temp.path(), probe).is_err() {
        return false;
    }
    Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(temp.path())
        .arg("/bin/echo")
        .arg("ok")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

macro_rules! skip_if_no_sandbox {
    () => {
        if !is_custom_sandbox_available() {
            eprintln!("Skipping test: sandbox-exec custom profiles unavailable");
            return;
        }
    };
}

/// Generate a sleep duration unlikely to collide with anything else on the
/// system, so we can identify our descendants in `ps` output. ~115 days is
/// clearly synthetic and highly unlikely to match an unrelated process.
fn unique_marker() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    9_000_000 + nanos % 1_000_000
}

/// Count running processes whose `ps -axo command` line contains `pattern`,
/// excluding the `ps` invocation itself.
fn count_processes(pattern: &str) -> usize {
    let output = Command::new("/bin/ps")
        .args(["-axo", "command"])
        .output()
        .expect("invoke ps");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains(pattern) && !line.contains("/bin/ps"))
        .count()
}

/// Best-effort: SIGKILL every process whose command line contains `pattern`.
/// Used as test cleanup to avoid leaking orphans across test runs.
fn pkill_pattern(pattern: &str) {
    let Ok(output) = Command::new("/bin/ps")
        .args(["-axo", "pid,command"])
        .output()
    else {
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.contains(pattern) || line.contains("/bin/ps") {
            continue;
        }
        let Some(pid_str) = line.split_whitespace().next() else {
            continue;
        };
        if let Ok(pid) = pid_str.parse::<i32>() {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

#[test]
fn test_sigterm_to_sx_propagates_to_sandbox_subtree() {
    skip_if_no_sandbox!();

    let marker = unique_marker();
    let pattern = format!("sleep {}", marker);
    let shell_cmd = format!("/bin/sleep {} & /bin/sleep {} & wait", marker, marker);

    let mut child = Command::new(SX_BIN)
        .arg("--no-config")
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg(&shell_cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sx");
    let sx_pid = child.id();

    // Give sandbox-exec → sh → sleep chain time to come up.
    thread::sleep(Duration::from_millis(800));
    let before = count_processes(&pattern);
    assert!(
        before >= 2,
        "expected at least 2 sleep descendants before SIGTERM, saw {}",
        before
    );

    unsafe {
        libc::kill(sx_pid as i32, libc::SIGTERM);
    }

    // Issue #37 acceptance criterion: zero descendants remain after 2s.
    // The 3s total budget covers SIGTERM → grace → SIGKILL plus reaping.
    thread::sleep(Duration::from_secs(3));
    let after = count_processes(&pattern);

    // Always reap and cleanup before asserting so a failure does not leak processes.
    let _ = child.wait();
    if after != 0 {
        pkill_pattern(&pattern);
    }

    assert_eq!(
        after, 0,
        "expected sandbox subtree to be gone 3s after SIGTERM, {} stragglers remain",
        after
    );
}

#[test]
fn test_sigkill_to_sx_orphans_subtree_known_limitation() {
    skip_if_no_sandbox!();

    // SIGKILL is uncatchable; sx has no opportunity to forward it. This test
    // pins down the limitation: `kill_on_drop` / signal handlers do not help
    // when sx itself is force-killed. A future supervisor process could fix
    // this — when it lands, this test will start failing and should be updated.
    let marker = unique_marker();
    let pattern = format!("sleep {}", marker);
    let shell_cmd = format!("/bin/sleep {}", marker);

    let mut child = Command::new(SX_BIN)
        .arg("--no-config")
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg(&shell_cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sx");
    let sx_pid = child.id();

    thread::sleep(Duration::from_millis(800));
    let before = count_processes(&pattern);
    assert!(
        before >= 1,
        "expected sleep descendant to be running before SIGKILL, saw {}",
        before
    );

    unsafe {
        libc::kill(sx_pid as i32, libc::SIGKILL);
    }
    let _ = child.wait();

    // Brief settle; orphan is reparented to launchd but stays alive.
    thread::sleep(Duration::from_millis(500));
    let after_kill = count_processes(&pattern);

    // Cleanup orphans before asserting, regardless of outcome.
    pkill_pattern(&pattern);

    assert!(
        after_kill >= 1,
        "expected SIGKILL to leave at least one orphan (uncatchable signal). \
         If this fails, supervisor logic has improved — update this test."
    );
}
