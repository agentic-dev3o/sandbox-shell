pub mod cli;
pub mod config;
pub mod detection;
pub mod sandbox;
pub mod shell;
pub mod utils;

use anyhow::Result;
use cli::args::Args;

pub fn run() -> Result<()> {
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
