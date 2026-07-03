use std::fs::{File, set_permissions};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use ropey::RopeSlice;
use tempfile::NamedTempFile;

pub fn hard_save(path: impl AsRef<Path>, append: bool, rope_slice: RopeSlice) -> io::Result<()> {
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

    let write_result = (|| save(path, append, rope_slice))();

    if let Some(ref perms) = original_perms {
        if perms.readonly() {
            let _ = set_permissions(path, perms.clone());
        }
    }

    write_result
}

pub fn save(path: impl AsRef<Path>, append: bool, rope_slice: RopeSlice) -> io::Result<()> {
    let path = path.as_ref();
    let parent = path.parent();

    if let Some(parent_path) = parent {
        std::fs::create_dir_all(parent_path)?;
    }

    let parent_path = parent.unwrap_or_else(|| Path::new("."));
    let temp = NamedTempFile::new_in(parent_path)?;
    let mut writer = BufWriter::new(temp.reopen()?);

    // This makes save not atomic, but oh well... (tm)
    if append {
        if let Ok(mut existing) = File::open(path) {
            io::copy(&mut existing, &mut writer)?;
        }
    }

    for chunk in rope_slice.chunks() {
        writer.write_all(chunk.as_bytes())?;
    }

    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);

    temp.persist(path).map_err(|e| e.error)?;

    Ok(())
}
