use goblin::{error, Object};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

mod extract;
mod util;
use util::fail;

// fn getit<'a>(path: &Path) -> Result<goblin::mach::MachO<'a>, error::Error> {
fn getit<'a>(buffer: &'a Vec<u8>) -> Result<goblin::mach::MachO<'a>, error::Error> {
    // let buffer = fs::read(path.to_str().unwrap())?;
    match Object::parse(&buffer)? {
        Object::Elf(elf) => {
            fail("elf");
        }
        Object::PE(pe) => {
            fail("PE");
        }
        Object::Mach(mach) => {
            // println!("mach: {:#?}", &mach);
            match mach {
                goblin::mach::Mach::Fat(fat) => {
                    for arch in fat.iter_arches() {
                        let arch = arch.unwrap();
                        // println!("here!: {:#?}", arch);

                        let nested = goblin::mach::MachO::parse(&buffer, arch.offset as usize)?;
                        return Ok(nested);
                        // doit(nested)?;
                        // println!("nested: {:#?} {:#?}", nested.libs, nested.rpaths);
                        // // nested.libs
                        // for a in nested.load_commands {
                        //     if let goblin::mach::load_command::CommandVariant::LoadDylib(
                        //         cmd,
                        //     ) = a.command
                        //     {
                        //         println!("cmd: {:#?}", cmd.dylib.name);
                        //     }
                        //     // println!("a: {:#?}", a);
                        // }

                        // break;
                    }

                    fail("nope");
                }
                goblin::mach::Mach::Binary(binary) => {
                    // println!("binary: {:#?}", binary);
                    // fail("needed");
                    return Ok(binary);
                }
            }
        }
        Object::Archive(archive) => {
            println!("archive: {:#?}", &archive);
            fail("archive");
        }
        Object::Unknown(magic) => {
            println!("unknown magic: {:#x}", magic);
            fail("uknown");
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
) -> Result<HashSet<String>, error::Error> {
    let buffer = fs::read(bin_path).unwrap();
    let nested = getit(&buffer).unwrap();
    // println!("nested: {:#?} {:#?}", nested.libs, nested.rpaths);
    // nested.libs

    let prefix = " ".repeat(indent);
    // println!("{}{}:", prefix, canonical_path.to_str().unwrap());
    let mut new_visited = visited.clone();
    for lib in nested.libs {
        // The LC_ID_DYLIB load command is contained in this list, so we need to skip the current
        // dylib to not get stuck in an infinite loop
        if lib == "self" || lib == canonical_path {
            continue;
        }

        if should_ignore(&lib, ignore_prefixes) {
            continue;
        }

        if visited.contains(&lib.to_owned()) {
            println!("{}{}", prefix, lib); // TODO Configurable
            continue;
        }

        new_visited.insert(lib.to_owned());

        let mut found = false;
        for path in get_potential_paths(shared_cache_root, &bin_path, &lib, &nested.rpaths) {
            if path.exists() {
                println!("{}{}:", prefix, lib);
                let nested_visit = doit(
                    shared_cache_root,
                    &path,
                    lib,
                    indent + 2,
                    &new_visited,
                    ignore_prefixes,
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
    let target_path = Path::new("/tmp/testlibs2");
    if !target_path.exists() {
        extract::extract_libs(target_path);
    }

    for (i, arg) in env::args().enumerate() {
        if i == 1 {
            let bin_path = Path::new(&arg);
            let visited = HashSet::new();
            println!("{}:", &arg);
            doit(
                target_path.to_str().unwrap(),
                &bin_path,
                &arg,
                2,
                &visited,
                &vec![],
                // &vec!["/usr/lib/swift".into(), "/System/Library".into()],
            )?;
        }
    }
    Ok(())
}
