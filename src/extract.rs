use block::{Block, ConcreteBlock};
use libloading::{Library, Symbol};
use std::collections::hash_map::DefaultHasher;
use std::ffi::CString;
use std::hash::{Hash, Hasher};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use crate::failf;
use crate::verbose_log;

pub fn extract_libs(shared_cache_path: Option<PathBuf>, verbose: bool) -> PathBuf {
    if let Some(shared_cache_path) = &shared_cache_path {
        if !shared_cache_path.exists() {
            failf!(
                "error: passed shared cache path doesn't exist: {}",
                shared_cache_path.to_string_lossy()
            );
        }
    }

    let potential_paths = vec![
        shared_cache_path,
        Some(Path::new(
            "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e",
        ).to_path_buf()),
        Some(Path::new(
            "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_x86_64h",
        ).to_path_buf()),
    ];

    let shared_cache_path = potential_paths
        .into_iter()
        .flatten()
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            failf!(
                "error: failed to find shared cache, please provide the path with --shared-cache-path and file an issue so we can add the new path"
            )
        });

    let mut output_path = PathBuf::from("/tmp/dylibtree");
    let mut hasher = DefaultHasher::new();
    shared_cache_path.hash(&mut hasher);
    let hash_str = format!("{:x}", hasher.finish());
    output_path.push(hash_str);

    verbose_log!(verbose, "Using shared cache at: {:?}", shared_cache_path);
    verbose_log!(verbose, "Extracted shared cache to: {:?}", output_path);
    if output_path.exists() {
        return output_path;
    }

    let success = extract_shared_cache(get_extractor_path(), &shared_cache_path, &output_path);
    if !success {
        _ = std::fs::remove_dir_all(output_path);
        failf!("error: failed to extract shared cache, see above for the error from dyld")
    }

    output_path
}

fn get_extractor_path() -> PathBuf {
    let output = std::process::Command::new("xcrun")
        .arg("--sdk")
        .arg("iphoneos")
        .arg("--show-sdk-platform-path")
        .output()
        .unwrap();
    if output.status.success() {
        let mut path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        path.push("usr/lib/dsc_extractor.bundle");

        if !path.exists() {
            failf!(
                "error: dsc_extractor.bundle wasn't found at expected path, Xcode might have changed this location: {}, please file an issue",
                path.to_str().unwrap()
            );
        }

        path
    } else {
        failf!("error: failed to fetch platform path from xcrun")
    }
}

fn path_to_cstring(path: &Path) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).unwrap()
}

fn extract_shared_cache(
    extractor_path: PathBuf,
    shared_cache_path: &Path,
    output_path: &Path,
) -> bool {
    if !shared_cache_path.exists() {
        failf!(
            "error: shared cache doesn't exist at path: {}",
            shared_cache_path.to_str().unwrap()
        );
    }

    let progress_block = ConcreteBlock::new(|x, y| eprintln!("extracted {}/{}", x, y));

    unsafe {
        let library = Library::new(extractor_path).unwrap();
        let func: Symbol<
            unsafe extern "C" fn(
                shared_cache_path: *const c_char,
                output_path: *const c_char,
                progress: &Block<(usize, usize), ()>,
            ) -> i32,
        > = library
            .get(b"dyld_shared_cache_extract_dylibs_progress")
            .unwrap();
        func(
            path_to_cstring(shared_cache_path).as_ptr(),
            path_to_cstring(output_path).as_ptr(),
            &progress_block,
        ) == 0
    }
}
