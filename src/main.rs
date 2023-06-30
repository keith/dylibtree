use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use goblin::mach::load_command;
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

fn print_dylib_paths(
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
    let buffer = fs::read(actual_path)?;
    let binary = load_binary(actual_path, &buffer)?;

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

fn newest_path_in_dir<F>(dir: &Path, matching: F) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .filter(|x| matching(x.file_name().to_str().unwrap()))
            .map(|x| x.path())
            .collect();
        paths.sort();
        paths.pop()
    } else {
        None
    }
}

// /Library/Developer/CoreSimulator/Volumes/iOS_*/Library/Developer/CoreSimulator/Profiles/Runtimes/.*.simruntime/Contents/Resources/RuntimeRoot
fn newest_simulator_path(os: &str) -> PathBuf {
    let root = Path::new("/Library/Developer/CoreSimulator/Volumes");
    if let Some(path) = newest_path_in_dir(root, |x| x.starts_with(os)) {
        let runtimes = path.join("Library/Developer/CoreSimulator/Profiles/Runtimes");
        if let Some(path) = newest_path_in_dir(&runtimes, |x| x.ends_with(".simruntime")) {
            let runtime_root = path.join("Contents/Resources/RuntimeRoot");
            if runtime_root.is_dir() {
                return runtime_root;
            }
        }
    }

    failf!(
        "error: no simulator runtimes found in '{}' for platform '{}'",
        root.display(),
        os
    );
}

// ~/Library/Developer/Xcode/iOS DeviceSupport/.*/Symbols
fn newest_device_path(os: &str) -> PathBuf {
    let device_support = PathBuf::from(std::env::var("HOME").unwrap())
        .join("Library/Developer/Xcode")
        .join(format!("{} DeviceSupport", os));
    if let Some(path) = newest_path_in_dir(&device_support, |_| true) {
        return path.join("Symbols");
    }

    failf!(
        "error: no device support directory found in '{}'",
        device_support.display(),
    );
}

// - For macOS extract the shared cache
// - For simulators the runtime root can be found in /Library/Developer/CoreSimulator/Volumes
// - For devices the symbols (good enough for this use) can be found in ~/Library/Developer/Xcode/iOS DeviceSupport
fn runtime_root_for_binary(binary_path: &Path, verbose: bool) -> Result<PathBuf, error::Error> {
    let buffer = &fs::read(binary_path)?;
    let initial_binary = load_binary(binary_path, buffer)?;

    // TODO: unhandled platforms:
    // pub const PLATFORM_TVOS: u32 = 3;
    // pub const PLATFORM_WATCHOS: u32 = 4;
    // pub const PLATFORM_BRIDGEOS: u32 = 5;
    // pub const PLATFORM_MACCATALYST: u32 = 6;
    // pub const PLATFORM_DRIVERKIT: u32 = 10;
    // xrOS but no constants yet
    for lc in initial_binary.load_commands {
        if let load_command::CommandVariant::BuildVersion(version) = lc.command {
            return match version.platform {
                load_command::PLATFORM_MACOS => {
                    let potential_paths = vec![
                        Path::new("/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e").to_path_buf(),
                        Path::new("/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_x86_64h").to_path_buf(),
                    ];
                    Ok(extract::extract_libs(potential_paths, verbose))
                }
                load_command::PLATFORM_IOS => Ok(newest_device_path("iOS")),
                load_command::PLATFORM_IOSSIMULATOR => Ok(newest_simulator_path("iOS")),
                load_command::PLATFORM_TVOSSIMULATOR => Ok(newest_simulator_path("tvOS")),
                load_command::PLATFORM_WATCHOSSIMULATOR => Ok(newest_simulator_path("watchOS")),
                _ => {
                    failf!("error: unsupported platform (id: {}), pass --runtime-root or --shared-cache-path, and please file an issue so we can add support", version.platform);
                }
            };
        }
    }

    failf!("error: no build version load command found in binary, pass --runtime-root or --shared-cache-path");
}

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
        extract::extract_libs(vec![path], args.verbose)
    } else {
        runtime_root_for_binary(&args.binary, args.verbose)?
    };

    let visited = HashSet::new();
    verbose_log!(args.verbose, "runtime_root: {:?}", runtime_root);
    print_dylib_paths(
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
