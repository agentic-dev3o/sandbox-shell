//! End-to-end Linux sandbox tests.
//!
//! These drive the real `sx` binary rather than the library, because the Linux
//! backend re-execs `sx` itself as its sandbox helper - the same path a user
//! exercises. The macOS equivalents live in `tests/integration.rs`.
#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const SX_BIN: &str = env!("CARGO_BIN_EXE_sx");

fn sx(dir: &Path, args: &[&str]) -> Output {
    Command::new(SX_BIN)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run sx")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// Ask sx itself whether the kernel enforces Landlock.
fn landlock_available() -> bool {
    let out = sx(Path::new("/tmp"), &["--dry-run", "--", "true"]);
    stdout(&out).contains("kernel Landlock ABI")
}

macro_rules! require_landlock {
    () => {
        if !landlock_available() {
            eprintln!("skipping: this kernel does not enforce Landlock");
            return;
        }
    };
}

/// A workspace outside `/tmp`, which the base profile deliberately makes
/// readable and writable. Home is denied by default, so a temp dir there gives
/// a genuine "outside the sandbox" location.
fn workspace() -> TempDir {
    let home = dirs::home_dir().expect("home directory");
    TempDir::new_in(home).expect("create workspace")
}

/// `<root>/work` is the working directory; `<root>/outside` must stay unreachable.
fn split_workspace() -> (TempDir, PathBuf, PathBuf) {
    let root = workspace();
    let work = root.path().join("work");
    let outside = root.path().join("outside");
    fs::create_dir(&work).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "TOPSECRET").unwrap();
    (root, work, outside)
}

#[test]
fn runs_a_command_and_propagates_its_exit_code() {
    require_landlock!();
    let root = workspace();

    let out = sx(root.path(), &["--", "/usr/bin/printf", "sandboxed"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "sandboxed");

    let out = sx(root.path(), &["--", "/usr/bin/bash", "-c", "exit 42"]);
    assert_eq!(out.status.code(), Some(42));
}

#[test]
fn reports_missing_and_non_executable_commands() {
    require_landlock!();
    let root = workspace();

    let out = sx(root.path(), &["--", "/usr/bin/sx-does-not-exist"]);
    assert_eq!(out.status.code(), Some(127));

    let not_executable = root.path().join("data.txt");
    fs::write(&not_executable, "not a program").unwrap();
    let out = sx(root.path(), &["--", not_executable.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(126));
}

#[test]
fn allows_reading_and_writing_inside_the_working_directory() {
    require_landlock!();
    let (_root, work, _outside) = split_workspace();
    fs::write(work.join("input.txt"), "hello").unwrap();

    let out = sx(&work, &["--", "/usr/bin/cat", "input.txt"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "hello");

    let out = sx(&work, &["--", "/usr/bin/touch", "created.txt"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(work.join("created.txt").exists());
}

#[test]
fn denies_reading_outside_the_allowlist() {
    require_landlock!();
    let (_root, work, outside) = split_workspace();

    let out = sx(
        &work,
        &[
            "--",
            "/usr/bin/cat",
            outside.join("secret.txt").to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "reading outside the sandbox succeeded"
    );
    assert!(!stdout(&out).contains("TOPSECRET"));
}

#[test]
fn denies_writing_outside_the_working_directory() {
    require_landlock!();
    let (_root, work, outside) = split_workspace();
    let target = outside.join("planted.txt");

    let out = sx(&work, &["--", "/usr/bin/touch", target.to_str().unwrap()]);
    assert!(
        !out.status.success(),
        "writing outside the sandbox succeeded"
    );
    assert!(!target.exists());
}

#[test]
fn restrictions_are_inherited_by_child_processes() {
    require_landlock!();
    let (_root, work, outside) = split_workspace();
    let secret = outside.join("secret.txt");

    // bash forks cat: the policy must survive both fork and exec.
    let out = sx(
        &work,
        &[
            "--",
            "/usr/bin/bash",
            "-c",
            &format!("cat {}", secret.display()),
        ],
    );
    assert!(!out.status.success());
    assert!(!stdout(&out).contains("TOPSECRET"));
}

#[test]
fn deny_read_carves_out_the_secret_but_keeps_siblings_readable() {
    require_landlock!();
    let (root, work, _outside) = split_workspace();
    // Deliberately a sibling of the working directory: the working directory
    // itself outranks deny_read (see the test below), so nesting the fixture
    // inside it would not exercise the carve-out.
    let home = root.path().join("home");
    let secret = home.join("private");
    let public = home.join("public");
    fs::create_dir_all(&secret).unwrap();
    fs::create_dir_all(&public).unwrap();
    fs::write(secret.join("key"), "TOPSECRET").unwrap();
    fs::write(public.join("readme"), "PUBLIC").unwrap();

    let allow = home.to_str().unwrap();
    let deny = secret.to_str().unwrap();

    // The sibling stays readable.
    let out = sx(
        &work,
        &[
            "--allow-read",
            allow,
            "--deny-read",
            deny,
            "--",
            "/usr/bin/cat",
            public.join("readme").to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "PUBLIC");

    // The denied file does not.
    let out = sx(
        &work,
        &[
            "--allow-read",
            allow,
            "--deny-read",
            deny,
            "--",
            "/usr/bin/cat",
            secret.join("key").to_str().unwrap(),
        ],
    );
    assert!(!out.status.success(), "denied file was readable");
    assert!(!stdout(&out).contains("TOPSECRET"));

    // Listing the parent still works, so `ls ~` is not broken by a deny.
    let out = sx(
        &work,
        &[
            "--allow-read",
            allow,
            "--deny-read",
            deny,
            "--",
            "/usr/bin/ls",
            allow,
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("public"));
}

#[test]
fn deny_read_wins_over_an_explicit_allow_of_the_same_path() {
    require_landlock!();
    let (root, work, _outside) = split_workspace();
    let vault = root.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::write(vault.join("key"), "TOPSECRET").unwrap();

    let out = sx(
        &work,
        &[
            "--allow-read",
            vault.to_str().unwrap(),
            "--deny-read",
            vault.to_str().unwrap(),
            "--",
            "/usr/bin/cat",
            vault.join("key").to_str().unwrap(),
        ],
    );
    assert!(
        !out.status.success(),
        "deny_read did not override allow_read"
    );
    assert!(!stdout(&out).contains("TOPSECRET"));
}

/// Mirrors Seatbelt, where the working-directory `(allow file* ...)` rule is
/// emitted after the deny block and therefore wins.
#[test]
fn the_working_directory_outranks_deny_read() {
    require_landlock!();
    let (_root, work, _outside) = split_workspace();
    let nested = work.join("vault");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("key"), "PROJECT_DATA").unwrap();

    let out = sx(
        &work,
        &[
            "--deny-read",
            nested.to_str().unwrap(),
            "--",
            "/usr/bin/cat",
            nested.join("key").to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "PROJECT_DATA");
}

/// True when sx can use a network namespace here; distributions that block
/// unprivileged user namespaces make it fall back to a seccomp filter.
fn uses_network_namespace() -> bool {
    let out = sx(Path::new("/tmp"), &["--dry-run", "--", "true"]);
    stdout(&out).contains("private network namespace")
}

/// The property that matters, whichever mechanism ends up enforcing it.
#[test]
fn offline_mode_blocks_network_access() {
    require_landlock!();
    let root = workspace();

    let program = "import socket, sys\n\
                   try:\n\
                   \x20   s = socket.socket(); s.settimeout(5); s.connect((\"1.1.1.1\", 53))\n\
                   \x20   print(\"CONNECTED\")\n\
                   except OSError as e:\n\
                   \x20   print(\"BLOCKED\", e)\n";

    let out = sx(root.path(), &["--", "/usr/bin/python3", "-c", program]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("BLOCKED"),
        "offline mode did not block the network: {}",
        stdout(&out)
    );
}

#[test]
fn offline_mode_uses_an_empty_network_namespace() {
    require_landlock!();
    if !uses_network_namespace() {
        eprintln!("skipping: sx fell back to seccomp, so there is no namespace to inspect");
        return;
    }
    let root = workspace();

    let out = sx(root.path(), &["--", "/usr/bin/cat", "/proc/net/dev"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let interfaces = interface_names(&stdout(&out));
    assert_eq!(
        interfaces,
        vec!["lo".to_string()],
        "offline sandbox should only see loopback"
    );
}

#[test]
fn online_mode_keeps_the_host_network() {
    require_landlock!();
    let root = workspace();

    let host = interface_names(&fs::read_to_string("/proc/net/dev").unwrap());
    if host.len() <= 1 {
        eprintln!("skipping: host itself has no non-loopback interface");
        return;
    }

    let out = sx(
        root.path(),
        &["online", "--", "/usr/bin/cat", "/proc/net/dev"],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(interface_names(&stdout(&out)), host);
}

#[test]
fn localhost_mode_allows_private_loopback_only() {
    require_landlock!();
    let root = workspace();

    let program = "import socket\n\
                   s = socket.socket(); s.bind((\"127.0.0.1\", 0)); s.listen(1)\n\
                   c = socket.socket(); c.connect((\"127.0.0.1\", s.getsockname()[1]))\n\
                   print(\"LOOPBACK_OK\")\n";

    let out = sx(
        root.path(),
        &["localhost", "--", "/usr/bin/python3", "-c", program],
    );
    if !out.status.success() && stderr(&out).contains("user namespace") {
        eprintln!("skipping: unprivileged user namespaces are disabled here");
        return;
    }
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("LOOPBACK_OK"));

    // Offline mode must not gain loopback as a side effect.
    let out = sx(root.path(), &["--", "/usr/bin/python3", "-c", program]);
    assert!(!out.status.success(), "offline mode reached loopback");
}

#[test]
fn no_new_privs_is_always_set() {
    require_landlock!();
    let root = workspace();

    let out = sx(
        root.path(),
        &["--", "/usr/bin/grep", "NoNewPrivs", "/proc/self/status"],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("NoNewPrivs:\t1"),
        "expected NoNewPrivs=1, got {:?}",
        stdout(&out)
    );
}

#[test]
fn dry_run_renders_the_landlock_policy_without_running_anything() {
    let root = workspace();
    let marker = root.path().join("side-effect");

    let out = sx(
        root.path(),
        &[
            "--dry-run",
            "--",
            "/usr/bin/touch",
            marker.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let policy = stdout(&out);
    assert!(policy.contains("# sx sandbox policy (landlock)"));
    assert!(policy.contains("r = read files + execute"));
    assert!(!marker.exists(), "--dry-run executed the command");
}

#[test]
fn explain_reports_the_landlock_backend() {
    let root = workspace();
    let out = sx(root.path(), &["--explain"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let text = stdout(&out);
    assert!(text.contains("Backend: landlock"));
    assert!(text.contains("no_new_privs"));
}

#[test]
fn the_sandbox_helper_refuses_incomplete_invocations() {
    let out = Command::new(SX_BIN)
        .arg("--sandbox-apply")
        .output()
        .expect("failed to run sx");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("usage"));

    // Without the policy in the environment there is nothing to enforce, so the
    // helper must refuse rather than exec the command unsandboxed.
    let out = Command::new(SX_BIN)
        .args(["--sandbox-apply", "/usr/bin/true"])
        .env_remove("SX_SANDBOX_SPEC")
        .output()
        .expect("failed to run sx");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("SX_SANDBOX_SPEC"));

    let out = Command::new(SX_BIN)
        .args(["--sandbox-apply", "/usr/bin/true"])
        .env("SX_SANDBOX_SPEC", "this is not valid toml {{{")
        .output()
        .expect("failed to run sx");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("sandbox spec"));
}

/// The policy must not leak into the sandboxed program's environment, and it
/// must not be handed over through a file the sandbox itself can write to.
#[test]
fn the_policy_is_not_visible_to_the_sandboxed_program() {
    require_landlock!();
    let root = workspace();

    let out = sx(root.path(), &["--", "/usr/bin/env"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        !stdout(&out).contains("SX_SANDBOX_SPEC"),
        "the sandboxed program inherited the policy"
    );
}

/// Interface names from a `/proc/net/dev` dump, in file order.
fn interface_names(proc_net_dev: &str) -> Vec<String> {
    proc_net_dev
        .lines()
        .skip(2)
        .filter_map(|line| line.split(':').next())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}
