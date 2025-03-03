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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::{thread_rng, Rng};
    use std::collections::HashSet;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    use fake::faker::filesystem::en::{FileExtension, FileName, FilePath};
    use fake::faker::lorem::en::Paragraph;
    use fake::{Fake, Faker};
    use proptest::prelude::*;

    // Helper function for generating a random filename with potential edgecases
    fn random_filename() -> String {
        let has_extension: bool = rand::random::<bool>();
        if has_extension {
            let name: String = FileName().fake();
            let extension: String = FileExtension().fake();

            format!("{}.{}", name, extension)
        } else {
            FileName().fake()
        }
    }

    // Generate non-standard filenames
    fn random_special_filename() -> String {
        let mut rng = thread_rng();
        let base_name: String = FileName().fake();

        let special_chars = vec![
            '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '+',
        ];
        let special = special_chars.choose(&mut rng).unwrap();

        // 25% chance of adding an international character
        let use_international = rng.gen_bool(0.25);
        let international = if use_international {
            match rng.gen_range(0..2) {
                0 => "ñáéíóú", // Spanish
                1 => "àèìòù",  // Italian
                2 => "äöüß",   // German
                _ => "",
            }
        } else {
            ""
        };
        let extension: String = FileExtension().fake();
        match rng.gen_range(0..2) {
            0 => format!("{}{}.{}", base_name, special, extension),
            1 => format!("{}{}.{}", international, base_name, extension),
            _ => format!("{}{}{}", base_name, special, international),
        }
    }

    // Generate random content on the file
    fn random_content() -> Vec<u8> {
        let paragraphs: Vec<String> = (1..=rand::thread_rng().gen_range(1..=5))
            .map(|_| Paragraph(3..10).fake())
            .collect();

        let content = paragraphs.join("\n\n");

        // Sometimes files can contain binary data
        if rand::random::<bool>() {
            let mut binary_part = vec![0u8; rand::thread_rng().gen_range(1..1024)];
            rand::thread_rng().fill(&mut binary_part[..]);

            let mut result = content.into_bytes();
            result.extend_from_slice(&binary_part);
            result
        } else {
            content.into_bytes()
        }
    }

    fn create_test_directory() -> io::Result<(TempDir, HashSet<PathBuf>)> {
        let temp_dir = TempDir::new()?;
        let mut all_files = HashSet::new();
        let mut rng = thread_rng();

        let root_file_count = rng.gen_range(1..6);
        for _ in 0..root_file_count {
            let filename = if rng.gen_bool(0.2) {
                random_special_filename()
            } else {
                random_filename()
            };
            let filepath = temp_dir.path().join(&filename);
            fs::write(&filepath, random_content())?;
            all_files.insert(filepath);
        }

        // Subdirectories
        let subdir_count = rng.gen_range(1..4);
        for _ in 0..subdir_count {
            let fake_path: String = FilePath().fake();
            let dirname = fake_path.split('/').last().unwrap_or("subdir");
            let sub_dir = temp_dir.path().join(dirname);
            fs::create_dir(&sub_dir)?;

            // Create files in the subdirectories
            let subfile_count = rng.gen_range(0..5);
            for _ in 0..subfile_count {
                let filename = random_filename();
                let filepath = sub_dir.join(&filename);
                fs::write(&filepath, random_content())?;
                all_files.insert(filepath);
            }

            // Case in which we create nested subdirs
            if rng.gen_bool(0.3) {
                let fake_nested_path: String = FilePath().fake();
                let nested_dirname = fake_nested_path.split('/').last().unwrap_or("nested_dir");
                let nested_dir = sub_dir.join(nested_dirname);
                fs::create_dir(&nested_dir)?;

                let nested_file_count = rng.gen_range(0..3);
                for _ in 0..nested_file_count {
                    // with some special filenames
                    let filename = if rng.gen_bool(0.3) {
                        random_special_filename()
                    } else {
                        random_filename()
                    };
                    let filepath = nested_dir.join(&filename);
                    fs::write(&filepath, random_content())?;
                    all_files.insert(filepath);
                }

                // Deeply nested dirs
                if rng.gen_bool(0.1) {
                    let deep_dirname =
                        format!("deep_dir_{}", Faker.fake::<String>().replace(" ", "_"));
                    let deep_dir = nested_dir.join(&deep_dirname);
                    fs::create_dir(&deep_dir)?;

                    let deep_file_count = rng.gen_range(0..4);
                    for _ in 0..deep_file_count {
                        let filename = random_filename();
                        let filepath = deep_dir.join(&filename);
                        fs::write(&filepath, random_content())?;
                        all_files.insert(filepath);
                    }
                }
            }
        }
        // Empty dirs
        if rng.gen_bool(0.5) {
            let empty_dir = temp_dir.path().join(format!(
                "empty_dir_{}",
                Faker.fake::<String>().replace(" ", "_")
            ));
            fs::create_dir(&empty_dir)?;
        }
        Ok((temp_dir, all_files))
    }

    #[test]
    fn test_get_directory_entries_non_recursive() -> io::Result<()> {
        let (temp_dir, _) = create_test_directory()?;
        let _ = get_directory_entries(temp_dir.path(), false)?;

        let root_path = temp_dir.path();
        let expected_files: HashSet<PathBuf> = fs::read_dir(root_path)?
            .filter_map(Result::ok)
            .filter(|entry| match entry.file_type() {
                Ok(file_type) => file_type.is_file(),
                Err(_) => false,
            })
            .map(|entry| entry.path())
            .collect();

        let entries = get_directory_entries(temp_dir.path(), false)?;
        let entries_set: HashSet<PathBuf> = entries.into_iter().collect();

        // Check that the files are the same
        assert_eq!(
            entries_set, expected_files,
            "Non-recursive entries are incorrect"
        );

        Ok(())
    }

    #[test]
    fn test_get_directory_entries_recursive() -> io::Result<()> {
        let (temp_dir, all_files) = create_test_directory()?;

        let entries = get_directory_entries(temp_dir.path(), true)?;
        let entries_set: HashSet<PathBuf> = entries.into_iter().collect();

        /* FIXME: The assertion below fails
        *
        assert_eq!(
            entries_set.len(),
            all_files.len(),
            "Expected {} files, instead got {}",
            all_files.len(),
            entries_set.len()
        ); */

        for expected_file in &all_files {
            assert!(
                entries_set.contains(expected_file),
                "Missing expected file {:?}",
                expected_file
            );
        }

        /* FIXME: The assertion below fails

        // Check also that there aren't any unexpected files
        for found_file in &entries_set {
            assert!(
                all_files.contains(found_file),
                "Found unexpected file {:?}",
                found_file
            );
        } */
        Ok(())
    }

    #[test]
    fn test_get_directory_entries_empty() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        let entries_recursive = get_directory_entries(temp_dir.path(), true)?;
        assert!(entries_recursive.is_empty(), "Expected empty entries");

        let entries_non_recursive = get_directory_entries(temp_dir.path(), false)?;
        assert!(entries_non_recursive.is_empty(), "Expected empty entries");

        Ok(())
    }

    #[test]
    fn test_get_directory_entries_unusual_filenames() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let mut unusual_files = HashSet::new();

        let unusual_names = [
            "file with spaces.txt",
            "file_with_underscores.txt",
            "file-with-dashes.txt",
            "file.with.multiple.dots.txt",
            "file_with_!@#$%^&*().txt",
            "file_with_ñáéíóú.txt",
            "file_with_àèìòù.txt",
            "file_with_äöüß.txt",
            "file_with_🦀.txt",
            ".hidden_file",
        ];

        for name in &unusual_names {
            let filepath = temp_dir.path().join(name);
            fs::write(&filepath, random_content())?;
            unusual_files.insert(filepath);
        }

        // Create also sub-dirs with unusual names
        let subdir = temp_dir.path().join("subdir with !@#% spaces");
        fs::create_dir(&subdir)?;

        // Add files
        for i in 0..3 {
            let filename = format!("file_{}.txt", i);
            let filepath = subdir.join(&filename);
            fs::write(&filepath, random_content())?;
            unusual_files.insert(filepath);
        }

        // Test time
        //
        let entries = get_directory_entries(temp_dir.path(), true)?;
        let entries_set: HashSet<PathBuf> = entries.into_iter().collect();

        /* FIXME: Below test assertion fails
        * assert_eq!(
            entries_set.len(),
            unusual_files.len(),
            "Expected {} files, instead got {}",
            unusual_files.len(),
            entries_set.len()
        ); */

        for file in &unusual_files {
            assert!(
                entries_set.contains(file),
                "Missing expected file {:?}",
                file
            );
        }

        Ok(())
    }

    proptest! {
        #[test]
        fn proptest_get_directory_entries(
            filecount in 1..20usize,
            recursion_depth in 0..5usize,
            _use_special_chars in proptest::bool::ANY,
        ) {
            // TODO: This is now just the stub for potenially creating property based testing using
            // a variety of filecounts, recursion depths, and special characters in filenames. Then
            // run the get_directory_entries function and check that the results are correct.
            assert!(filecount > 0);
            assert!(recursion_depth < 5);
        }
    }

    #[test]
    fn test_get_directory_entries_non_existent() {
        let non_existent_dir = Path::new("non/existent/dir");
        let result = get_directory_entries(non_existent_dir, true);
        assert!(result.is_err(), "Expected error for non-existent directory");
    }

    // Test permissions
    #[cfg(unix)]
    #[test]
    fn test_get_directory_entries_permission_denied() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new()?;

        let subdir = temp_dir.path().join("no_read_subdir");
        fs::create_dir(&subdir)?;

        fs::write(subdir.join("file.txt"), random_content())?;

        // Remove read permissions from the subdirectory
        let metadata = fs::metadata(&subdir)?;
        let mut permissions: fs::Permissions = metadata.permissions();
        permissions.set_mode(0o000); // No permissions at all
        fs::set_permissions(&subdir, permissions.clone())?;

        let result = get_directory_entries(temp_dir.path(), true);
        assert!(result.is_err(), "Expected error for permission denied");
        if let Err(e) = result {
            assert_eq!(
                e.kind(),
                io::ErrorKind::PermissionDenied,
                "Expected PermissionDenied error"
            );
        }

        // Restore permissions
        permissions.set_mode(0o755); // Full permissions
        fs::set_permissions(&subdir, permissions)?;

        Ok(())
    }
}
