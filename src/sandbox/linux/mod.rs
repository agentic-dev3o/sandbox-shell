//! Linux sandbox backend: Landlock for the filesystem, namespaces (or seccomp)
//! for the network.

pub mod apply;
pub mod landlock;
pub mod net;
pub mod policy;
pub mod rules;
