//! Filesystem helpers.

use std::fs::{self, File};
use std::path::Path;

use crate::error::MediaResult;

/// Writes `dir/name` through a temporary `name.part` that is renamed into
/// place only after `write` succeeds, so a partial file never appears under
/// the final name. The temporary file is removed on any failure.
pub(crate) fn write_atomically(
    dir: &Path,
    name: &str,
    write: impl FnOnce(&mut File) -> MediaResult<()>,
) -> MediaResult<()> {
    let dest = dir.join(name);
    let tmp = dir.join(format!("{name}.part"));

    let result = (|| {
        let mut file = File::create(&tmp)?;
        write(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, &dest)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MediaLibError;

    fn scratch() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("media_lib_fs_{}_{:?}", std::process::id(), std::thread::current().id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn success_renames_into_place() {
        let dir = scratch();
        write_atomically(&dir, "ok.enc", |f| {
            use std::io::Write;
            f.write_all(b"data")?;
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(dir.join("ok.enc")).unwrap(), b"data");
        assert!(!dir.join("ok.enc.part").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failure_leaves_nothing_behind() {
        let dir = scratch();
        let result = write_atomically(&dir, "bad.enc", |_| Err(MediaLibError::TrailingData));
        assert!(matches!(result, Err(MediaLibError::TrailingData)));
        assert!(!dir.join("bad.enc").exists());
        assert!(!dir.join("bad.enc.part").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
