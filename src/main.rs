use std::collections::HashSet;

use goblin::error;

mod binary;
mod cli;
mod dyld_shared_cache;
mod print;
mod runtime_root;
#[macro_use]
mod util;

fn main() -> Result<(), error::Error> {
    unsafe {
        // https://github.com/rust-lang/rust/issues/46016#issuecomment-428106774
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = cli::parse_args();
    let runtime_root = if let Some(path) = args.runtime_root {
        path
    } else if let Some(path) = args.shared_cache_path {
        if !path.exists() {
            failf!(
                "error: passed shared cache path doesn't exist: {}",
                path.to_string_lossy()
            );
        }
        dyld_shared_cache::extract_libs(vec![path], args.verbose)
    } else {
        runtime_root::runtime_root_for_binary(&args.binary, args.verbose)?
    };

    let visited = HashSet::new();
    verbose_log!(args.verbose, "runtime_root: {:?}", runtime_root);
    print::print_dylib_paths(
        &runtime_root,
        &args.binary,
        args.binary.to_str().unwrap(),
        0,
        args.depth,
        &visited,
        &args.ignore_prefixes,
        args.exclude_all_duplicates,
        args.verbose,
    )?;
    Ok(())
}
