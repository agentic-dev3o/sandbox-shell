pub mod cli;
pub mod config;
pub mod detection;
pub mod sandbox;
pub mod shell;
pub mod utils;

use anyhow::Result;
use cli::args::Args;

pub fn run() -> Result<()> {
    // The Linux backend re-execs this binary as its sandbox launcher. Handle
    // that before clap runs so the target command's arguments are passed
    // through verbatim and never reinterpreted as sx flags.
    #[cfg(target_os = "linux")]
    {
        let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
        if argv.get(1).and_then(|a| a.to_str()) == Some(sandbox::backend::APPLY_FLAG) {
            sandbox::linux::apply::run(&argv[2..]);
        }
    }

    let args = Args::parse_args();

    if args.init {
        return cli::commands::init_config();
    }

    if args.explain {
        return cli::commands::explain(&args);
    }

    if args.dry_run {
        return cli::commands::dry_run(&args);
    }

    cli::commands::execute(&args)
}
