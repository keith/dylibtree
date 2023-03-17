use block::{Block, ConcreteBlock};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use crate::util;

pub fn extract_libs(target_path: &Path) {
    let library_path = get_library_path();
    let input_path = path_to_cstring2(Path::new(
        "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e",
    ));
    let input_path = path_to_cstring2(Path::new(
            "/Users/ksmiley/Library/Developer/Xcode/iOS DeviceSupport/16.4 (20E5212f) arm64e/Symbols/private/preboot/Cryptexes/OS/System/Library/Caches/com.apple.dyld/dyld_shared_cache_arm64e"
    ));

    let output_path = path_to_cstring2(target_path);

    extract_shared_cache(library_path, input_path, output_path);
}

fn get_library_path() -> PathBuf {
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
            util::fail(format!(
            "error: dsc_extractor.bundle wasn't found at expected path, Xcode might have changed this location: {}",
            path.to_str().unwrap()
        ));
        }

        dbg!(&path);
        path
    } else {
        util::fail("error: failed to fetch platform path from xcrun")
    }
}

fn path_to_cstring2(path: &Path) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).unwrap()
}

fn extract_shared_cache(library_path: PathBuf, input_path: CString, output_path: CString) {
    let progress_block = ConcreteBlock::new(|x, y| println!("extracted {}/{}", x, y));
    unsafe {
        let library = Library::new(library_path).unwrap();
        let func: Symbol<
            unsafe extern "C" fn(
                input_path: *const c_char,
                output_path: *const c_char,
                progress: &Block<(usize, usize), ()>,
            ),
        > = library
            .get(b"dyld_shared_cache_extract_dylibs_progress")
            .unwrap();
        func(input_path.as_ptr(), output_path.as_ptr(), &progress_block);
    }
}
