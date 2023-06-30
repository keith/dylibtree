use std::collections::HashSet;
use std::path::{Path, PathBuf};

use goblin::error;

use crate::binary;
use crate::verbose_log;

pub fn print_dylib_paths(
    shared_cache_root: &PathBuf,
    actual_path: &Path,
    canonical_path: &str,
    depth: usize,
    max_depth: usize,
    visited: &HashSet<String>,
    ignore_prefixes: &Vec<String>,
    exclude_all_duplicates: bool,
    verbose: bool,
) -> Result<HashSet<String>, error::Error> {
    let buffer = std::fs::read(actual_path)?;
    let binary = binary::load_binary(actual_path, &buffer)?;

    verbose_log!(verbose, "Visiting lib: {:?}", actual_path);
    let indent = depth * 2;
    println!("{}{}:", " ".repeat(indent), canonical_path);
    let prefix = " ".repeat(indent + 2);
    let mut visited = visited.clone();
    for dylib in binary.libs {
        // The LC_ID_DYLIB load command is contained in this list, so we need to skip the current
        // dylib to not get stuck in an infinite loop
        if dylib == "self" || dylib == canonical_path {
            continue;
        }

        if depth + 1 > max_depth {
            continue;
        }

        if should_ignore(dylib, ignore_prefixes) {
            verbose_log!(verbose, "Ignoring prefix: {}", dylib);
            continue;
        }

        if visited.contains(&dylib.to_owned()) {
            if !exclude_all_duplicates {
                println!("{}{}", prefix, dylib);
            }
            continue;
        }

        visited.insert(dylib.to_owned());

        let mut found = false;
        for path in get_potential_paths(shared_cache_root, actual_path, dylib, &binary.rpaths) {
            verbose_log!(verbose, "Checking path: {:?}", path);
            if path.exists() {
                verbose_log!(verbose, "Found path: {:?}", path);
                visited.extend(print_dylib_paths(
                    shared_cache_root,
                    &path,
                    dylib,
                    depth + 1,
                    max_depth,
                    &visited,
                    ignore_prefixes,
                    exclude_all_duplicates,
                    verbose,
                )?);
                found = true;
                break;
            }
        }

        if !found {
            println!("{}{}: warning: not found", prefix, dylib);
        }
    }

    Ok(visited)
}

fn get_potential_paths(
    shared_cache_root: &PathBuf,
    executable_path: &Path,
    lib: &str,
    rpaths: &Vec<&str>,
) -> Vec<PathBuf> {
    let mut paths = vec![];

    if lib.starts_with("@rpath/") {
        let lib = lib.split_once('/').unwrap().1;
        for rpath in rpaths {
            // TODO: @loader_path/ isn't right here, but this is better than nothing for now
            if rpath.starts_with("@executable_path/") || rpath.starts_with("@loader_path/") {
                let rpath = rpath.split_once('/').unwrap().1;
                let mut path = PathBuf::from(executable_path.parent().unwrap());
                path.push(rpath);
                path.push(lib);
                paths.push(path);
                continue;
            }

            let mut path = PathBuf::from(shared_cache_root);
            let stripped = rpath.strip_prefix('/').unwrap();
            path.push(stripped);
            path.push(lib);
            paths.push(path);

            let mut path = PathBuf::from(rpath);
            path.push(lib);
            paths.push(path);
        }
    } else {
        let mut path = PathBuf::from(shared_cache_root);
        let stripped = lib.strip_prefix('/').unwrap();
        path.push(stripped);
        paths.push(path);

        paths.push(Path::new(lib).to_path_buf());
    }

    paths
}

fn should_ignore(lib: &str, ignore_prefixes: &Vec<String>) -> bool {
    for prefix in ignore_prefixes {
        if lib.starts_with(prefix) {
            return true;
        }
    }

    false
}
