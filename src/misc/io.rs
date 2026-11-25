use std::fs::{File, set_permissions};
use std::io::{self, BufWriter, Write};
use std::panic;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::components::BufferSlice;
use crate::rope::RopeInfo;

pub fn hard_save(
    path: impl AsRef<Path>,
    append: bool,
    buf_slice: BufferSlice,
) -> io::Result<RopeInfo> {
    let path = path.as_ref();

    let original_perms = match path.metadata() {
        Ok(metadata) => Some(metadata.permissions()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    if let Some(ref perms) = original_perms {
        if perms.readonly() {
            let mut new_perms = perms.clone();
            #[allow(clippy::permissions_set_readonly_false)]
            new_perms.set_readonly(false);
            set_permissions(path, new_perms)?;
        }
    }

    let write_result = save(path, append, buf_slice);

    if let Some(ref perms) = original_perms {
        if perms.readonly() {
            let _ = set_permissions(path, perms.clone());
        }
    }

    write_result
}

pub fn save(path: impl AsRef<Path>, append: bool, buf_slice: BufferSlice) -> io::Result<RopeInfo> {
    let path = path.as_ref();
    let parent = path.parent();

    if let Some(parent_path) = parent {
        std::fs::create_dir_all(parent_path)?;
    }

    let parent_path = parent.unwrap_or_else(|| Path::new("."));
    let temp = NamedTempFile::new_in(parent_path)?;
    let mut writer = BufWriter::new(temp.reopen()?);

    // This makes save non atomic, but oh well...
    if append {
        if let Ok(mut existing) = File::open(path) {
            io::copy(&mut existing, &mut writer)?;
        }
    }

    let mut num_bytes: usize = 0;
    let mut num_lines: usize = 0;

    for line in buf_slice.lines() {
        num_lines = num_lines.saturating_add(1);
        for chunk in line.chunks() {
            writer.write_all(chunk.as_bytes())?;
            num_bytes = num_bytes.saturating_add(chunk.len());
        }
    }

    if num_bytes == 0 {
        num_lines = 0;
    }

    if buf_slice.should_append_nl() {
        writer.write(&['\n' as u8])?;
        num_bytes = num_bytes.saturating_add(1);
    }

    writer.flush()?;
    writer.get_ref().sync_all()?;

    temp.persist(path).map_err(|e| e.error)?;

    Ok(RopeInfo {
        num_lines,
        num_bytes,
    })
}
