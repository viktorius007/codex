use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::TryRngCore;
use rand::rngs::OsRng;

use crate::OpenError;

const KEY_FILE_NAME: &str = ".fingerprint-key";
const KEY_BYTES: usize = 32;

pub(crate) fn load_or_create(directory: &Path) -> Result<[u8; KEY_BYTES], OpenError> {
    fs::create_dir_all(directory).map_err(OpenError::from_io)?;
    let path = directory.join(KEY_FILE_NAME);
    match read_key(&path) {
        Ok(key) => {
            ensure_private_file(&path).map_err(OpenError::from_io)?;
            Ok(key)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => publish_new_key(directory, &path),
        Err(error) => Err(OpenError::from_io(error)),
    }
}

pub(crate) fn random_id() -> Result<String, OpenError> {
    random_bytes::<16>().map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
}

fn publish_new_key(directory: &Path, path: &Path) -> Result<[u8; KEY_BYTES], OpenError> {
    let key = random_bytes()?;
    let temporary_id = random_id()?;
    let temporary_path = directory.join(format!(".{KEY_FILE_NAME}.{temporary_id}.tmp"));
    let publish_result = write_and_publish(&temporary_path, path, &key);

    match publish_result {
        Ok(()) => {
            sync_directory(directory).map_err(OpenError::from_io)?;
            fs::remove_file(&temporary_path).map_err(OpenError::from_io)?;
            sync_directory(directory).map_err(OpenError::from_io)?;
            ensure_private_file(path).map_err(OpenError::from_io)?;
            Ok(key)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            sync_directory(directory).map_err(OpenError::from_io)?;
            fs::remove_file(&temporary_path).map_err(OpenError::from_io)?;
            sync_directory(directory).map_err(OpenError::from_io)?;
            let existing = read_key(path).map_err(OpenError::from_io)?;
            ensure_private_file(path).map_err(OpenError::from_io)?;
            Ok(existing)
        }
        Err(error) => {
            fs::remove_file(&temporary_path).map_err(OpenError::from_io)?;
            Err(OpenError::from_io(error))
        }
    }
}

fn write_and_publish(temporary_path: &Path, path: &Path, key: &[u8]) -> io::Result<()> {
    let mut file = open_private_create_new(temporary_path)?;
    file.write_all(key)?;
    file.sync_all()?;
    drop(file);
    fs::hard_link(temporary_path, path)
}

fn read_key(path: &Path) -> io::Result<[u8; KEY_BYTES]> {
    fs::read(path)?.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "cache diagnostic fingerprint key has an invalid length",
        )
    })
}

pub(crate) fn open_private_create_new(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn ensure_private_file(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cache diagnostic fingerprint key is not a regular file",
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        if permissions.mode() & 0o777 != 0o600 {
            permissions.set_mode(0o600);
            fs::set_permissions(path, permissions)?;
        }
    }
    Ok(())
}

fn random_bytes<const N: usize>() -> Result<[u8; N], OpenError> {
    let mut bytes = [0_u8; N];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| OpenError::unavailable())?;
    Ok(bytes)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}
