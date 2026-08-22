//! Human-readable rendering of the Linux policy, for `sx --dry-run`.
//!
//! The Seatbelt backend prints the profile it hands to `sandbox-exec`; this is
//! the equivalent for Landlock - the resolved rule set, plus the parts of the
//! configuration that Linux enforces differently.

use crate::config::schema::{ExecSugid, NetworkMode};
use crate::sandbox::linux::landlock::Support;
use crate::sandbox::linux::net;
use crate::sandbox::linux::rules::{self, Rule, LIST, READ, WRITE};
use crate::sandbox::params::SandboxParams;
use std::fmt::Write;

/// Render the policy `sx` would enforce for these parameters.
pub fn render(params: &SandboxParams) -> String {
    let support = Support::detect();
    let out_rules = rules::build(params);
    render_with(params, &out_rules, support, net::namespace_usable(false))
}

/// Rendering split from probing so it can be tested deterministically.
fn render_with(
    params: &SandboxParams,
    out_rules: &[Rule],
    support: Support,
    userns: bool,
) -> String {
    let mut out = String::new();

    out.push_str("# sx sandbox policy (landlock)\n");
    for note in notes_with(params, support, userns) {
        let _ = writeln!(out, "# {}", note);
    }

    if !params.deny_read.is_empty() {
        out.push_str("\n# denied for reading (no rule is emitted for these paths)\n");
        for path in &params.deny_read {
            let _ = writeln!(out, "# deny {}", path.display());
        }
    }

    out.push_str("\n# r = read files + execute, l = list directory, w = create/modify/delete\n");
    for rule in out_rules {
        let _ = writeln!(out, "{} {}", access_str(rule.access), rule.path.display());
    }

    out
}

/// Short statements about what this kernel will and will not enforce.
///
/// Shared by `--dry-run` (as comment lines) and `--explain`.
pub fn notes(params: &SandboxParams) -> Vec<String> {
    notes_with(params, Support::detect(), net::namespace_usable(false))
}

fn notes_with(params: &SandboxParams, support: Support, userns: bool) -> Vec<String> {
    let mut notes = Vec::new();

    if support.available() {
        notes.push(format!("kernel Landlock ABI: {}", support.abi));
        for gap in support.missing() {
            notes.push(format!("not enforced on this kernel: {}", gap));
        }
    } else {
        notes.push(
            "WARNING: this kernel does not support Landlock; sx will refuse to run".to_string(),
        );
    }

    notes.push(format!(
        "network: {} -> {}",
        mode_name(params.network_mode),
        network_plan(params.network_mode, userns)
    ));
    notes.push("setuid/setgid execution: never elevates (no_new_privs is always set)".to_string());

    if let ExecSugid::Paths(paths) = &params.allow_exec_sugid {
        if !paths.is_empty() {
            notes.push(format!(
                "note: allow_exec_sugid has no effect on Linux (ignored: {})",
                paths.join(", ")
            ));
        }
    }
    if params.raw_rules.is_some() {
        notes.push("note: raw seatbelt rules are ignored on Linux".to_string());
    }
    if !params.deny_read.is_empty() {
        notes.push(
            "note: deny_read removes file-content access; directory names under a denied \
             path may still be listable when a parent grants listing"
                .to_string(),
        );
    }

    notes
}

fn access_str(access: u8) -> String {
    let mut s = String::with_capacity(3);
    s.push(if access & READ != 0 { 'r' } else { '-' });
    s.push(if access & LIST != 0 { 'l' } else { '-' });
    s.push(if access & WRITE != 0 { 'w' } else { '-' });
    s
}

fn mode_name(mode: NetworkMode) -> &'static str {
    match mode {
        NetworkMode::Offline => "offline",
        NetworkMode::Online => "online",
        NetworkMode::Localhost => "localhost",
    }
}

fn network_plan(mode: NetworkMode, userns: bool) -> String {
    match (mode, userns) {
        (NetworkMode::Online, _) => "unrestricted".to_string(),
        (NetworkMode::Offline, true) => "private network namespace".to_string(),
        (NetworkMode::Offline, false) => "seccomp fallback (no usable user namespace)".to_string(),
        (NetworkMode::Localhost, true) => "private network namespace with loopback".to_string(),
        (NetworkMode::Localhost, false) => {
            "UNAVAILABLE - no usable user namespace on this system".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample() -> SandboxParams {
        SandboxParams {
            working_dir: PathBuf::from("/home/u/app"),
            network_mode: NetworkMode::Offline,
            allow_read: vec![PathBuf::from("/usr")],
            ..Default::default()
        }
    }

    #[test]
    fn renders_access_bits() {
        assert_eq!(access_str(READ | LIST | WRITE), "rlw");
        assert_eq!(access_str(READ), "r--");
        assert_eq!(access_str(LIST), "-l-");
        assert_eq!(access_str(WRITE), "--w");
        assert_eq!(access_str(0), "---");
    }

    #[test]
    fn lists_rules_and_working_dir() {
        let params = sample();
        let out = render_with(&params, &rules::build(&params), Support { abi: 9 }, true);
        assert!(out.contains("rl- /usr"));
        assert!(out.contains("rlw /home/u/app"));
        assert!(out.contains("kernel Landlock ABI: 9"));
    }

    #[test]
    fn warns_when_landlock_is_missing() {
        let params = sample();
        let out = render_with(&params, &[], Support { abi: 0 }, true);
        assert!(out.contains("does not support Landlock"));
    }

    #[test]
    fn reports_seccomp_fallback_without_user_namespaces() {
        let params = sample();
        let out = render_with(&params, &[], Support { abi: 9 }, false);
        assert!(out.contains("seccomp fallback"));
    }

    #[test]
    fn localhost_without_user_namespaces_is_flagged_unavailable() {
        let params = SandboxParams {
            network_mode: NetworkMode::Localhost,
            ..sample()
        };
        let out = render_with(&params, &[], Support { abi: 9 }, false);
        assert!(out.contains("UNAVAILABLE"));
    }

    #[test]
    fn denied_paths_are_listed_for_auditing() {
        let params = SandboxParams {
            deny_read: vec![PathBuf::from("/home/u/.aws")],
            ..sample()
        };
        let out = render_with(&params, &[], Support { abi: 9 }, true);
        assert!(out.contains("# deny /home/u/.aws"));
    }

    #[test]
    fn notes_ignored_macos_only_settings() {
        let params = SandboxParams {
            allow_exec_sugid: ExecSugid::Paths(vec!["/bin/ps".into()]),
            raw_rules: Some("(allow foo)".into()),
            ..sample()
        };
        let out = render_with(&params, &[], Support { abi: 9 }, true);
        assert!(out.contains("allow_exec_sugid has no effect on Linux"));
        assert!(out.contains("raw seatbelt rules are ignored"));
    }

    #[test]
    fn old_kernel_gaps_are_listed() {
        let params = sample();
        let out = render_with(&params, &[], Support { abi: 1 }, true);
        assert!(out.contains("signal scoping"));
    }
}
