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
    #[arg(short, long, value_name = "PREFIX")]
    pub ignore_prefixes: Vec<String>,
}

pub fn parse_args() -> Args {
    return Args::parse();
}
