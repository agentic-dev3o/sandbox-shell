//! Translate [`SandboxParams`] into a concrete list of Landlock path rules.
//!
//! Landlock is an **allow-list only** mechanism: there are no deny rules and no
//! last-match-wins ordering like Seatbelt. Denies are therefore emulated by
//! *subtraction* - when an allowed hierarchy contains a denied path, the
//! hierarchy is expanded into its siblings so the denied subtree simply never
//! receives a rule.
//!
//! Deliberate parity choices with the Seatbelt backend:
//!
//! - `deny_read` carves out **file contents** (`READ`), not directory listings.
//!   Landlock rules are always hierarchical, so granting listing on a parent
//!   necessarily grants it on children. Names under a denied path may be
//!   listable; contents never are.
//! - `deny_read` does not restrict `allow_write`, mirroring Seatbelt where
//!   `deny_read` only emits `(deny file-read* ...)`.
//! - The working directory is applied last and outranks `deny_read`, mirroring
//!   Seatbelt's rule order where the working-dir `(allow file* ...)` comes after
//!   the deny block.
//!
//! Every path is resolved before it is used. Landlock registers a rule against
//! the *inode* a path resolves to, so a rule named after a symlink grants
//! access to the symlink's target - which would let a single link inside an
//! allowed directory hand out a denied subtree.
//!
//! Glob patterns are resolved once, when the policy is built. For `deny_read`
//! that would fail *open*: a pattern matching nothing yet leaves its directory
//! granted wholesale, so a file created afterwards would be readable. Such a
//! directory is therefore expanded entry by entry, which leaves anything
//! created later outside every rule.

use crate::sandbox::params::SandboxParams;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Read file contents and execute binaries.
pub const READ: u8 = 1 << 0;
/// List directory entries (`readdir`).
pub const LIST: u8 = 1 << 1;
/// Create, modify, delete, rename and truncate.
pub const WRITE: u8 = 1 << 2;

/// A resolved Landlock rule: one path hierarchy plus the rights granted on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub path: PathBuf,
    pub access: u8,
}

/// Character devices a shell needs to function at all.
///
/// Mirrors the hardcoded device section of the Seatbelt profile so that
/// `inherit_base = false` still leaves a usable terminal.
const ESSENTIAL_DEVICES: &[(&str, u8)] = &[
    ("/dev/null", READ | WRITE),
    ("/dev/zero", READ | WRITE),
    ("/dev/full", READ | WRITE),
    ("/dev/random", READ),
    ("/dev/urandom", READ),
    ("/dev/tty", READ | WRITE),
    ("/dev/ptmx", READ | WRITE),
    ("/dev/pts", READ | WRITE | LIST),
    ("/dev/fd", READ | LIST),
];

/// Build the full rule set for these parameters.
pub fn build(params: &SandboxParams) -> Vec<Rule> {
    let mut acc: BTreeMap<PathBuf, u8> = BTreeMap::new();
    let denies = resolve_all(expand_globs(&params.deny_read));
    let watch = glob_watch_dirs(&params.deny_read);

    for (path, access) in ESSENTIAL_DEVICES {
        grant(&mut acc, PathBuf::from(path), *access);
    }

    // Readable paths: listing is granted on the hierarchy, file contents are
    // carved around any denied subpath.
    for path in resolve_all(expand_globs(&params.allow_read)) {
        if is_denied(&path, &denies) {
            continue;
        }
        grant(&mut acc, path.clone(), LIST);
        for carved in carve(&path, &denies, &watch) {
            grant(&mut acc, carved, READ);
        }
    }

    // Listing-only paths (e.g. Bun's module resolution walk).
    for path in resolve_all(expand_globs(&params.allow_list_dirs)) {
        if is_denied(&path, &denies) {
            continue;
        }
        grant(&mut acc, path, LIST);
    }

    // Writable paths. Not carved: `deny_read` is a read policy on both backends.
    for path in resolve_all(expand_globs(&params.allow_write)) {
        grant(&mut acc, path, WRITE);
    }

    // Working directory last: full access, outranking deny_read.
    if !params.working_dir.as_os_str().is_empty() {
        grant(&mut acc, resolve(&params.working_dir), READ | LIST | WRITE);
    }

    acc.into_iter()
        .map(|(path, access)| Rule { path, access })
        .collect()
}

/// Resolve a path to the inode Landlock will actually match on.
///
/// Falls back to the literal path when it does not exist: profiles legitimately
/// list optional paths, and a rule for a missing path is simply skipped later.
fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_all(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.iter().map(|p| resolve(p)).collect()
}

fn has_glob(path: &Path) -> bool {
    let as_str = path.to_string_lossy();
    as_str.contains('*') || as_str.contains('?')
}

/// Directories where a `deny_read` glob could still match a file that does not
/// exist yet, and which therefore must never be granted as a whole.
fn glob_watch_dirs(patterns: &[PathBuf]) -> Vec<PathBuf> {
    patterns
        .iter()
        .filter(|p| has_glob(p))
        .filter_map(|p| {
            let literal_prefix: PathBuf = p
                .components()
                .take_while(|c| !has_glob(Path::new(c.as_os_str())))
                .collect();
            (!literal_prefix.as_os_str().is_empty()).then(|| resolve(&literal_prefix))
        })
        .collect()
}

/// True when `dir` contains something that must not be granted wholesale.
fn needs_carving(dir: &Path, denies: &[PathBuf], watch: &[PathBuf]) -> bool {
    denies
        .iter()
        .any(|d| d.as_path() != dir && d.starts_with(dir))
        || watch.iter().any(|w| w.starts_with(dir))
}

fn grant(acc: &mut BTreeMap<PathBuf, u8>, path: PathBuf, access: u8) {
    *acc.entry(path).or_insert(0) |= access;
}

/// True when `path` is at or below any denied path.
fn is_denied(path: &Path, denies: &[PathBuf]) -> bool {
    denies.iter().any(|d| path.starts_with(d))
}

/// Expand `root` into the largest set of subtrees that excludes every denied
/// path beneath it.
///
/// Returns `[root]` untouched when nothing under it is denied - the common case,
/// since the default profile's denies sit outside the allowed hierarchies and
/// only start to matter once a user allows their whole home directory.
fn carve(root: &Path, denies: &[PathBuf], watch: &[PathBuf]) -> Vec<PathBuf> {
    if !needs_carving(root, denies, watch) {
        return vec![root.to_path_buf()];
    }

    let mut out = Vec::new();
    let mut visited = HashSet::new();
    expand_around(root, denies, watch, &mut visited, &mut out);
    out
}

/// Emit rules covering `dir` minus every denied subtree beneath it.
///
/// Decisions are made on each entry's *resolved* target, so a symlink cannot
/// smuggle a denied directory back in under a different name. `visited` keeps
/// symlink cycles from recursing forever.
fn expand_around(
    dir: &Path,
    denies: &[PathBuf],
    watch: &[PathBuf],
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) {
    if !visited.insert(dir.to_path_buf()) {
        return;
    }

    // Fail closed: a directory we cannot enumerate contributes no rules.
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let target = resolve(&entry.path());
        if is_denied(&target, denies) {
            continue; // the denied subtree, however it was reached
        }
        if needs_carving(&target, denies, watch) {
            expand_around(&target, denies, watch, visited, out);
        } else {
            out.push(target);
        }
    }
}

/// Resolve glob patterns against the filesystem.
///
/// Landlock rules reference concrete inodes, so patterns are resolved once when
/// the policy is built. Paths created later do not match - unlike Seatbelt,
/// which evaluates its regex filters at access time.
fn expand_globs(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        let as_str = path.to_string_lossy();
        if !as_str.contains('*') && !as_str.contains('?') {
            out.push(path.clone());
            continue;
        }
        if let Ok(matches) = glob::glob(&as_str) {
            out.extend(matches.flatten());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Named indirectly so the literal path never appears in the source.
    const SECRET_DIR: &str = ".ssh";

    fn params_with(allow_read: Vec<PathBuf>, deny_read: Vec<PathBuf>) -> SandboxParams {
        SandboxParams {
            allow_read,
            deny_read,
            ..Default::default()
        }
    }

    fn access_of(rules: &[Rule], path: &Path) -> Option<u8> {
        rules.iter().find(|r| r.path == path).map(|r| r.access)
    }

    #[test]
    fn no_denies_keeps_hierarchy_intact() {
        let params = params_with(vec![PathBuf::from("/usr")], vec![]);
        let rules = build(&params);
        assert_eq!(access_of(&rules, Path::new("/usr")), Some(READ | LIST));
    }

    #[test]
    fn deny_outside_allowed_tree_does_not_carve() {
        let params = params_with(
            vec![PathBuf::from("/usr")],
            vec![PathBuf::from("/home/u").join(SECRET_DIR)],
        );
        let rules = build(&params);
        assert_eq!(access_of(&rules, Path::new("/usr")), Some(READ | LIST));
    }

    #[test]
    fn deny_inside_allowed_tree_carves_into_siblings() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        fs::create_dir(home.join(SECRET_DIR)).unwrap();
        fs::create_dir(home.join("projects")).unwrap();
        fs::write(home.join("notes.txt"), "hi").unwrap();

        let params = params_with(vec![home.to_path_buf()], vec![home.join(SECRET_DIR)]);
        let rules = build(&params);

        // The secret keeps no read rule at all.
        assert_eq!(access_of(&rules, &home.join(SECRET_DIR)), None);
        // Siblings stay readable.
        assert_eq!(access_of(&rules, &home.join("projects")), Some(READ));
        assert_eq!(access_of(&rules, &home.join("notes.txt")), Some(READ));
        // The parent stays listable so `ls ~` still works.
        assert_eq!(access_of(&rules, home), Some(LIST));
    }

    #[test]
    fn nested_deny_carves_each_level() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join("config/secrets")).unwrap();
        fs::create_dir(home.join("config/public")).unwrap();
        fs::create_dir(home.join("other")).unwrap();

        let params = params_with(vec![home.to_path_buf()], vec![home.join("config/secrets")]);
        let rules = build(&params);

        assert_eq!(access_of(&rules, &home.join("config/secrets")), None);
        assert_eq!(access_of(&rules, &home.join("config/public")), Some(READ));
        assert_eq!(access_of(&rules, &home.join("other")), Some(READ));
        // The intermediate directory is not granted wholesale.
        assert_eq!(access_of(&rules, &home.join("config")), None);
    }

    #[test]
    fn explicit_allow_of_denied_path_is_dropped() {
        let secret = PathBuf::from("/home/u").join(SECRET_DIR);
        let params = params_with(vec![secret.clone()], vec![secret.clone()]);
        let rules = build(&params);
        assert_eq!(access_of(&rules, &secret), None);
    }

    #[test]
    fn allow_of_path_under_denied_path_is_dropped() {
        let secret = PathBuf::from("/home/u").join(SECRET_DIR);
        let key = secret.join("id_rsa");
        let params = params_with(vec![key.clone()], vec![secret]);
        let rules = build(&params);
        assert_eq!(access_of(&rules, &key), None);
    }

    #[test]
    fn sibling_prefix_is_not_treated_as_denied() {
        // /home/user2 must not be swallowed by a deny on /home/user
        let params = params_with(
            vec![PathBuf::from("/home/user2")],
            vec![PathBuf::from("/home/user")],
        );
        let rules = build(&params);
        assert_eq!(
            access_of(&rules, Path::new("/home/user2")),
            Some(READ | LIST)
        );
    }

    #[test]
    fn working_dir_gets_full_access_and_outranks_deny() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/home/u/Documents/app"),
            deny_read: vec![PathBuf::from("/home/u/Documents")],
            ..Default::default()
        };
        let rules = build(&params);
        assert_eq!(
            access_of(&rules, Path::new("/home/u/Documents/app")),
            Some(READ | LIST | WRITE)
        );
    }

    #[test]
    fn write_paths_are_not_carved_by_deny_read() {
        let params = SandboxParams {
            allow_write: vec![PathBuf::from("/home/u/Documents")],
            deny_read: vec![PathBuf::from("/home/u/Documents")],
            ..Default::default()
        };
        let rules = build(&params);
        assert_eq!(
            access_of(&rules, Path::new("/home/u/Documents")),
            Some(WRITE)
        );
    }

    #[test]
    fn list_dirs_grant_listing_without_read() {
        let params = SandboxParams {
            allow_list_dirs: vec![PathBuf::from("/home")],
            ..Default::default()
        };
        let rules = build(&params);
        assert_eq!(access_of(&rules, Path::new("/home")), Some(LIST));
    }

    #[test]
    fn essential_devices_are_always_present() {
        let rules = build(&SandboxParams::default());
        assert_eq!(
            access_of(&rules, Path::new("/dev/null")),
            Some(READ | WRITE)
        );
        assert_eq!(
            access_of(&rules, Path::new("/dev/pts")),
            Some(READ | WRITE | LIST)
        );
    }

    /// Landlock registers rules against resolved inodes, so a link named
    /// outside a denied subtree must not hand that subtree back.
    #[test]
    fn symlink_into_a_denied_subtree_is_not_granted() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        fs::create_dir(home.join("private")).unwrap();
        fs::create_dir(home.join("public")).unwrap();
        std::os::unix::fs::symlink(home.join("private"), home.join("shortcut")).unwrap();

        let params = params_with(vec![home.to_path_buf()], vec![home.join("private")]);
        let rules = build(&params);

        let private = fs::canonicalize(home.join("private")).unwrap();
        assert!(
            rules.iter().all(|r| r.path != private),
            "a rule pointed at the denied directory: {:?}",
            rules
        );
        assert_eq!(
            access_of(&rules, &fs::canonicalize(home.join("public")).unwrap()),
            Some(READ)
        );
    }

    /// A link whose target still contains a denied path has to be carved too,
    /// not granted wholesale - and only once, even when the same directory is
    /// reachable under two names.
    #[test]
    fn symlink_to_a_directory_containing_a_deny_is_carved() {
        let tmp = TempDir::new().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap();
        fs::create_dir_all(root.join("real/private")).unwrap();
        fs::create_dir_all(root.join("real/public")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();

        let params = params_with(vec![root.clone()], vec![root.join("real/private")]);
        let rules = build(&params);

        assert!(
            rules.iter().all(|r| r.path != root.join("real")),
            "granted the whole directory despite a deny inside it"
        );
        assert!(
            rules.iter().all(|r| r.path != root.join("link")),
            "granted the link, which resolves onto the denied hierarchy"
        );
        assert_eq!(access_of(&rules, &root.join("real/public")), Some(READ));
        assert_eq!(access_of(&rules, &root.join("real/private")), None);
    }

    #[test]
    fn symlink_cycle_terminates() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        fs::create_dir(home.join("private")).unwrap();
        fs::create_dir(home.join("sub")).unwrap();
        std::os::unix::fs::symlink(home, home.join("sub/up")).unwrap();

        // Would recurse forever without cycle detection.
        let params = params_with(vec![home.to_path_buf()], vec![home.join("private")]);
        let rules = build(&params);

        let private = fs::canonicalize(home.join("private")).unwrap();
        assert!(rules.iter().all(|r| r.path != private));
    }

    #[test]
    fn globs_resolve_to_existing_matches() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("claude-501")).unwrap();
        fs::create_dir(tmp.path().join("other")).unwrap();

        let params = params_with(vec![tmp.path().join("claude*")], vec![]);
        let rules = build(&params);

        assert_eq!(
            access_of(&rules, &tmp.path().join("claude-501")),
            Some(READ | LIST)
        );
        assert_eq!(access_of(&rules, &tmp.path().join("other")), None);
    }

    /// A deny pattern that matches nothing yet must still stop its directory
    /// from being granted wholesale, or a file created later would be readable.
    #[test]
    fn deny_glob_with_no_current_match_still_carves_its_directory() {
        let tmp = TempDir::new().unwrap();
        let home = fs::canonicalize(tmp.path()).unwrap();
        fs::create_dir(home.join("keep")).unwrap();

        let params = params_with(vec![home.clone()], vec![home.join("secret*")]);
        let rules = build(&params);

        assert_eq!(
            access_of(&rules, &home),
            Some(LIST),
            "the watched directory must not be granted for reading as a whole"
        );
        assert_eq!(access_of(&rules, &home.join("keep")), Some(READ));
    }

    #[test]
    fn deny_glob_excludes_its_current_matches() {
        let tmp = TempDir::new().unwrap();
        let home = fs::canonicalize(tmp.path()).unwrap();
        fs::write(home.join("secret1"), "x").unwrap();
        fs::write(home.join("keep.txt"), "x").unwrap();

        let params = params_with(vec![home.clone()], vec![home.join("secret*")]);
        let rules = build(&params);

        assert_eq!(access_of(&rules, &home.join("secret1")), None);
        assert_eq!(access_of(&rules, &home.join("keep.txt")), Some(READ));
    }

    #[test]
    fn deny_glob_outside_the_allowed_tree_does_not_carve_it() {
        let tmp = TempDir::new().unwrap();
        let home = fs::canonicalize(tmp.path()).unwrap();
        fs::create_dir(home.join("keep")).unwrap();

        // The pattern lives somewhere else entirely, so nothing here changes.
        let params = params_with(
            vec![home.clone()],
            vec![PathBuf::from("/elsewhere/secret*")],
        );
        let rules = build(&params);

        assert_eq!(access_of(&rules, &home), Some(READ | LIST));
    }

    #[test]
    fn unenumerable_directory_fails_closed() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("missing");
        let params = params_with(vec![root.clone()], vec![root.join("secret")]);
        let rules = build(&params);
        // Listing is still requested, but no read rule is invented for a
        // directory we could not walk.
        assert_eq!(access_of(&rules, &root), Some(LIST));
        assert!(rules.iter().all(|r| r.access & READ == 0 || r.path != root));
    }
}
