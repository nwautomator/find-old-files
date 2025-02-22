use chrono::prelude::*;
use std::io;
use std::{fs, time::SystemTime};

pub fn get_access_time(path: &std::path::Path) -> anyhow::Result<u64> {
    let metadata = fs::metadata(path)?;
    if let Ok(atime) = metadata.accessed() {
        Ok(atime
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Invalid access time")
            .as_secs())
    } else {
        Err(anyhow::anyhow!("Could not get access time for file",))
    }
}

pub fn get_directory_entries(
    directory: &std::path::Path,
    recursive: bool,
) -> io::Result<Vec<std::path::PathBuf>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if recursive {
                entries.extend(get_directory_entries(&entry.path(), true)?);
            } else {
                continue;
            }
        }
        entries.push(entry.path());
    }

    Ok(entries)
}

pub fn get_access_times(directory: &std::path::Path, recursive: bool) -> anyhow::Result<()> {
    let entries = get_directory_entries(directory, recursive)?;
    for entry in entries {
        if entry.is_file() {
            let accessed_time: DateTime<Utc> =
                DateTime::from_timestamp(get_access_time(&entry)? as i64, 0).unwrap();
            println!("{} - {:?}", entry.display(), accessed_time);
        }
    }

    Ok(())
}
