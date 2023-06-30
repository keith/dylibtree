use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path of the binary to print dylibs for
    #[arg(value_name = "BINARY")]
    pub binary: PathBuf,

    /// Exclude all duplicate dylib names, reduces output but excludes first level
    /// dependencies as well
    #[arg(short, long, default_value_t = false)]
    pub exclude_all_duplicates: bool,

    /// Exclude dylibs that start with these prefixes
    #[arg(short = 'p', long, value_name = "PREFIX")]
    pub ignore_prefix: Vec<String>,

    /// Path to the shared cache, if not provided try to discover it.
    #[arg(short, long)]
    pub shared_cache_path: Option<PathBuf>,

    /// Path to the a device or simulators root runtime directory containing the dylibs for
    /// the OS, if not provided try to discover it.
    #[arg(short, long)]
    pub runtime_root: Option<PathBuf>,

    /// The maximum depth of libraries to print. Reduce to reduce output
    #[arg(short, long, default_value_t = 9999)]
    pub depth: usize,

    /// Print verbose output for debugging dylibtree itself
    #[arg(long, default_value_t = false, hide = true)]
    pub verbose: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}
