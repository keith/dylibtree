use std::path::Path;

use goblin::{error, Object};

use crate::failf;

pub fn load_binary<'a>(
    path: &Path,
    buffer: &'a [u8],
) -> Result<goblin::mach::MachO<'a>, error::Error> {
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
