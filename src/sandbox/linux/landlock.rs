//! Landlock enforcement.
//!
//! Landlock is the closest Linux analogue to Seatbelt: an unprivileged,
//! per-process filesystem policy that survives `exec` and is inherited by every
//! descendant. Unlike a mount namespace it does not change the filesystem
//! *view* - denied paths still exist, they just return `EACCES`, which matches
//! Seatbelt's behaviour and keeps error messages recognisable.

use crate::sandbox::linux::rules::{Rule, LIST, READ, WRITE};
use landlock::{
    Access, AccessFs, BitFlags, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreatedAttr, RulesetStatus, Scope, ABI,
};

/// Access rights `sx` asks the kernel to enforce.
///
/// ABI v3 is the v1 file rights plus `Refer` (cross-directory rename/link, which
/// build tools need) and `Truncate`. `IoctlDev` (v5) is deliberately left
/// *unhandled* so device ioctls stay implicitly allowed - this mirrors the
/// Seatbelt profile's global `(allow file-ioctl)` and keeps terminal control
/// (`tcsetattr`, window resize) working inside an interactive shell.
const HANDLED: ABI = ABI::V3;

/// What the running kernel can actually enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Support {
    /// Landlock ABI level, or 0 when unsupported.
    pub abi: i32,
}

impl Support {
    /// Ask the kernel for its Landlock ABI level.
    ///
    /// `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` is
    /// the documented probe; it returns the ABI version or `-1` with `ENOSYS` /
    /// `EOPNOTSUPP` when Landlock is missing or disabled.
    pub fn detect() -> Self {
        const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1;
        let rc = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                std::ptr::null::<libc::c_void>(),
                0_usize,
                LANDLOCK_CREATE_RULESET_VERSION,
            )
        };
        Self {
            abi: if rc < 0 { 0 } else { rc as i32 },
        }
    }

    pub fn available(&self) -> bool {
        self.abi > 0
    }

    /// Rights the kernel understands but is too old to enforce.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut gaps = Vec::new();
        if self.abi > 0 && self.abi < 2 {
            gaps.push("cross-directory rename/link control (ABI 2)");
        }
        if self.abi > 0 && self.abi < 3 {
            gaps.push("truncate control (ABI 3)");
        }
        if self.abi > 0 && self.abi < 6 {
            gaps.push("signal scoping (ABI 6)");
        }
        gaps
    }
}

fn flags_for(access: u8) -> BitFlags<AccessFs> {
    let mut flags = BitFlags::<AccessFs>::EMPTY;
    if access & READ != 0 {
        flags |= AccessFs::Execute | AccessFs::ReadFile;
    }
    if access & LIST != 0 {
        flags |= AccessFs::ReadDir;
    }
    if access & WRITE != 0 {
        flags |= AccessFs::from_write(HANDLED);
    }
    flags
}

/// Apply `rules` to the calling process and every process it later execs.
///
/// Paths that do not exist are skipped: Landlock rules reference open file
/// descriptors, while profiles legitimately list optional paths such as
/// `~/.bashrc`. Seatbelt tolerates the same thing.
pub fn apply(rules: &[Rule]) -> Result<RulesetStatus, String> {
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(HANDLED))
        .map_err(|e| format!("failed to select Landlock access rights: {e}"))?
        // Matches the Seatbelt profile's `(allow signal (target self))`: the
        // sandbox may signal its own processes, nothing outside it.
        .scope(Scope::Signal)
        .map_err(|e| format!("failed to scope signals: {e}"))?
        .create()
        .map_err(|e| format!("failed to create Landlock ruleset: {e}"))?;

    for rule in rules {
        let access = flags_for(rule.access);
        if access.is_empty() {
            continue;
        }
        let Ok(fd) = PathFd::new(&rule.path) else {
            continue; // optional path that is not present on this machine
        };
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|e| format!("failed to add rule for {}: {e}", rule.path.display()))?;
    }

    let status = ruleset
        .restrict_self()
        .map_err(|e| format!("failed to enforce Landlock ruleset: {e}"))?;

    // Fail closed. A sandbox that silently does not sandbox is worse than none.
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(
            "Landlock is not enforced by this kernel. sx needs Linux 5.13+ built with \
             CONFIG_SECURITY_LANDLOCK=y and `landlock` present in /sys/kernel/security/lsm. \
             Refusing to run the command unsandboxed."
                .to_string(),
        );
    }
    if !status.no_new_privs {
        return Err("failed to set no_new_privs; refusing to run unsandboxed".to_string());
    }

    Ok(status.ruleset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_maps_to_execute_and_read_file() {
        let flags = flags_for(READ);
        assert!(flags.contains(AccessFs::ReadFile));
        assert!(flags.contains(AccessFs::Execute));
        assert!(!flags.contains(AccessFs::ReadDir));
        assert!(!flags.contains(AccessFs::WriteFile));
    }

    #[test]
    fn list_maps_to_read_dir_only() {
        let flags = flags_for(LIST);
        assert_eq!(flags, AccessFs::ReadDir);
    }

    #[test]
    fn write_includes_refer_and_truncate() {
        let flags = flags_for(WRITE);
        assert!(flags.contains(AccessFs::WriteFile));
        assert!(flags.contains(AccessFs::Refer));
        assert!(flags.contains(AccessFs::Truncate));
        assert!(!flags.contains(AccessFs::ReadFile));
    }

    #[test]
    fn ioctl_dev_is_left_unhandled_so_terminals_keep_working() {
        assert!(!AccessFs::from_all(HANDLED).contains(AccessFs::IoctlDev));
    }

    #[test]
    fn empty_access_produces_no_flags() {
        assert!(flags_for(0).is_empty());
    }

    #[test]
    fn support_detection_matches_running_kernel() {
        let support = Support::detect();
        assert!(support.abi >= 0);
        assert_eq!(support.available(), support.abi > 0);
        if support.abi >= 6 {
            assert!(support.missing().is_empty());
        }
    }
}
