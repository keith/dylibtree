use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use goblin::{error, Object};

mod cli;
mod extract;
mod util;
use util::fail;

fn getit<'a>(path: &Path, buffer: &'a Vec<u8>) -> Result<goblin::mach::MachO<'a>, error::Error> {
    match Object::parse(&buffer)? {
        Object::Mach(mach) => match mach {
            goblin::mach::Mach::Fat(fat) => {
                for arch in fat.iter_arches() {
                    return Ok(goblin::mach::MachO::parse(&buffer, arch?.offset as usize)?);
                }

                fail("nope");
            }
            goblin::mach::Mach::Binary(binary) => {
                return Ok(binary);
            }
        },
        Object::Archive(_) => {
            fail(format!(
                "{}: error: archives are not currently supported",
                path.to_string_lossy(),
            ));
        }
        Object::Elf(_) => {
            fail(format!(
                "{}: error: ELF binaries are not currently supported, use lddtree instead",
                path.to_string_lossy(),
            ));
        }
        Object::PE(_) => {
            fail(format!(
                "{}: error: PE binaries are not currently supported",
                path.to_string_lossy(),
            ));
        }
        Object::Unknown(magic) => {
            fail(format!(
                "{}: error: unknown file magic: {:#x}, please file an issue if this is a Mach-O file",
                path.to_string_lossy(),
                magic
            ));
        }
    }
}

fn get_potential_paths(
    shared_cache_root: &str,
    executable_path: &Path,
    lib: &str,
    rpaths: &Vec<&str>,
) -> Vec<PathBuf> {
    let mut paths = vec![];

    if lib.starts_with("@rpath/") {
        let lib = lib.splitn(2, "/").nth(1).unwrap();
        for rpath in rpaths {
            // TODO: @loader_path/ isn't right here, but this is better than nothing for now
            if rpath.starts_with("@executable_path/") || rpath.starts_with("@loader_path/") {
                let rpath = rpath.splitn(2, "/").nth(1).unwrap();
                let mut path = PathBuf::from(executable_path.parent().unwrap());
                path.push(&rpath);
                path.push(&lib);
                paths.push(path);
                continue;
            }

            let mut path = PathBuf::from(rpath);
            path.push(&lib);
            paths.push(path);

            let mut path = PathBuf::from(shared_cache_root);
            let rpath = rpath.strip_prefix("/").unwrap();
            path.push(rpath);
            path.push(&lib);
            paths.push(path);
        }
    } else {
        paths.push(Path::new(lib).to_path_buf());

        let mut path = PathBuf::from(shared_cache_root);
        let lib = lib.strip_prefix("/").unwrap();
        path.push(lib);
        paths.push(path);
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

fn doit(
    shared_cache_root: &str,
    bin_path: &Path,
    canonical_path: &str,
    indent: usize,
    visited: &HashSet<String>,
    ignore_prefixes: &Vec<String>,
    exclude_all_duplicates: bool,
) -> Result<HashSet<String>, error::Error> {
    let buffer = fs::read(bin_path).unwrap();
    let binary = getit(bin_path, &buffer).unwrap();

    println!("{}{}:", " ".repeat(indent), canonical_path);
    let prefix = " ".repeat(indent + 2);
    let mut new_visited = visited.clone();
    for lib in binary.libs {
        // The LC_ID_DYLIB load command is contained in this list, so we need to skip the current
        // dylib to not get stuck in an infinite loop
        if lib == "self" || lib == canonical_path {
            continue;
        }

        if should_ignore(&lib, ignore_prefixes) {
            continue;
        }

        if visited.contains(&lib.to_owned()) {
            if !exclude_all_duplicates {
                println!("{}{}", prefix, lib);
            }
            continue;
        }

        new_visited.insert(lib.to_owned());

        let mut found = false;
        for path in get_potential_paths(shared_cache_root, &bin_path, &lib, &binary.rpaths) {
            if path.exists() {
                let nested_visit = doit(
                    shared_cache_root,
                    &path,
                    lib,
                    indent + 2,
                    &new_visited,
                    ignore_prefixes,
                    exclude_all_duplicates,
                )?;
                new_visited.extend(nested_visit);
                found = true;
                break;
            }
        }

        if !found {
            println!("{}{}: warning: not found", prefix, lib);
        }
    }

    Ok(new_visited)
}

fn main() -> Result<(), error::Error> {
    let args = cli::parse_args();
    let target_path = Path::new("/tmp/testlibs2");
    if !target_path.exists() {
        extract::extract_libs(target_path);
    }

    let visited = HashSet::new();
    doit(
        target_path.to_str().unwrap(),
        &args.binary,
        args.binary.to_str().unwrap(),
        0,
        &visited,
        &args.ignore_prefixes,
        args.exclude_all_duplicates,
    )?;
    Ok(())
}
