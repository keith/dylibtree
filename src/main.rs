use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use goblin::{error, Object};

mod cli;
mod extract;
#[macro_use]
mod util;

fn load_binary<'a>(path: &Path, buffer: &'a [u8]) -> Result<goblin::mach::MachO<'a>, error::Error> {
    match Object::parse(buffer)? {
        Object::Mach(mach) => match mach {
            goblin::mach::Mach::Fat(fat) => {
                if let Some(arch) = fat.iter_arches().next() {
                    return goblin::mach::MachO::parse(buffer, arch?.offset as usize);
                }

                failf!(
                    "{}: error: no architectures found in fat binary, please file an issue if this is a valid Mach-O file",
                    path.to_string_lossy(),
                );
            }
            goblin::mach::Mach::Binary(binary) => Ok(binary),
        },
        Object::Archive(_) => {
            failf!(
                "{}: error: archives are not currently supported",
                path.to_string_lossy(),
            );
        }
        Object::Elf(_) => {
            failf!(
                "{}: error: ELF binaries are not currently supported, use lddtree instead",
                path.to_string_lossy(),
            );
        }
        Object::PE(_) => {
            failf!(
                "{}: error: PE binaries are not currently supported",
                path.to_string_lossy(),
            );
        }
        Object::Unknown(magic) => {
            failf!(
                "{}: error: unknown file magic: {:#x}, please file an issue if this is a Mach-O file",
                path.to_string_lossy(),
                magic,
            );
        }
    }
}

fn get_potential_paths(
    shared_cache_root: &Option<PathBuf>,
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

            let mut path = PathBuf::from(rpath);
            path.push(lib);
            paths.push(path);

            if let Some(shared_cache_root) = &shared_cache_root {
                let mut path = PathBuf::from(shared_cache_root);
                let rpath = rpath.strip_prefix('/').unwrap();
                path.push(rpath);
                path.push(lib);
                paths.push(path);
            }
        }
    } else {
        paths.push(Path::new(lib).to_path_buf());

        if let Some(shared_cache_root) = &shared_cache_root {
            let mut path = PathBuf::from(shared_cache_root);
            let lib = lib.strip_prefix('/').unwrap();
            path.push(lib);
            paths.push(path);
        }
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

fn print_dylib_paths(
    shared_cache_root: &Option<PathBuf>,
    actual_path: &Path,
    canonical_path: &str,
    indent: usize,
    visited: &HashSet<String>,
    ignore_prefixes: &Vec<String>,
    exclude_all_duplicates: bool,
) -> Result<HashSet<String>, error::Error> {
    let buffer = fs::read(actual_path).unwrap();
    let binary = load_binary(actual_path, &buffer).unwrap();

    println!("{}{}:", " ".repeat(indent), canonical_path);
    let prefix = " ".repeat(indent + 2);
    let mut visited = visited.clone();
    for dylib in binary.libs {
        // The LC_ID_DYLIB load command is contained in this list, so we need to skip the current
        // dylib to not get stuck in an infinite loop
        if dylib == "self" || dylib == canonical_path {
            continue;
        }

        if should_ignore(dylib, ignore_prefixes) {
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
            if path.exists() {
                visited.extend(print_dylib_paths(
                    shared_cache_root,
                    &path,
                    dylib,
                    indent + 2,
                    &visited,
                    ignore_prefixes,
                    exclude_all_duplicates,
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

fn main() -> Result<(), error::Error> {
    let args = cli::parse_args();
    let mut extracted_cache_path: Option<PathBuf> = None;
    if args.include_system_dependencies {
        extracted_cache_path = Some(extract::extract_libs(args.shared_cache_path));
    }

    let visited = HashSet::new();
    print_dylib_paths(
        &extracted_cache_path,
        &args.binary,
        args.binary.to_str().unwrap(),
        0,
        &visited,
        &args.ignore_prefixes,
        args.exclude_all_duplicates,
    )?;
    Ok(())
}
