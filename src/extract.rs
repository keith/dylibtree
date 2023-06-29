use block::{Block, ConcreteBlock};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use crate::failf;

pub fn extract_libs(output_path: &Path) {
    let extractor_path = get_extractor_path();
    let shared_cache_path = Path::new(
        "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e",
    );
    let shared_cache_path = Path::new(
            "/Users/ksmiley/Library/Developer/Xcode/iOS DeviceSupport/16.4 (20E5212f) arm64e/Symbols/private/preboot/Cryptexes/OS/System/Library/Caches/com.apple.dyld/dyld_shared_cache_arm64e"
    );

    extract_shared_cache(extractor_path, shared_cache_path, output_path);
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

fn extract_shared_cache(extractor_path: PathBuf, shared_cache_path: &Path, output_path: &Path) {
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
            ),
        > = library
            .get(b"dyld_shared_cache_extract_dylibs_progress")
            .unwrap();
        func(
            path_to_cstring(shared_cache_path).as_ptr(),
            path_to_cstring(output_path).as_ptr(),
            &progress_block,
        );
    }
}
