use std::path::{Path, PathBuf};

use goblin::error;
use goblin::mach::load_command;

use crate::binary;
use crate::dyld_shared_cache;
use crate::failf;

// - For macOS extract the shared cache
// - For simulators the runtime root can be found in /Library/Developer/CoreSimulator/Volumes
// - For devices the symbols (good enough for this use) can be found in ~/Library/Developer/Xcode/iOS DeviceSupport
pub fn runtime_root_for_binary(binary_path: &Path, verbose: bool) -> Result<PathBuf, error::Error> {
    let buffer = &std::fs::read(binary_path)?;
    let initial_binary = binary::load_binary(binary_path, buffer)?;

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
                    Ok(dyld_shared_cache::extract_libs(potential_paths, verbose))
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
