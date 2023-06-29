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

    /// Also print all dependent dylibs from system frameworks. This requires extracting the
    /// shared cache, and will result in a lot of output
    #[arg(short, long, default_value_t = false)]
    pub include_system_dependencies: bool,

    /// Exclude dylibs that start with these prefixes
    #[arg(short = 'p', long, value_name = "PREFIX")]
    pub ignore_prefixes: Vec<String>,

    /// Path to the shared cache, if not provided default to the systems, regardless of what
    /// platform the binary was built for.
    #[arg(short, long)]
    pub shared_cache_path: Option<PathBuf>,
}

pub fn parse_args() -> Args {
    return Args::parse();
}
