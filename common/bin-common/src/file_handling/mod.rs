use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

use filetime::FileTime;
use walkdir::WalkDir;

/// Keeps the most recently accessed directories in the given directory.
pub fn keep_recently_accessed_dirs<P: AsRef<Path>>(dir: P, num_to_keep: usize) -> io::Result<()> {
    let mut dir_access_times: HashMap<PathBuf, FileTime> = HashMap::new();

    for entry in std::fs::read_dir(dir)?.flatten() {
        if entry.file_type().map(|e| e.is_dir()).unwrap_or(false) {
            // For each sub directory, keep track of the most recently accessed time of any
            // file inside that sub directory.
            let sub_dir = entry.path();
            let mut most_recent_access_time: Option<FileTime> = None;

            for file_entry in WalkDir::new(&sub_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let metadata = file_entry.metadata().unwrap();
                let accessed_time = FileTime::from_last_access_time(&metadata);

                if most_recent_access_time.map_or(true, |current| accessed_time > current) {
                    most_recent_access_time = Some(accessed_time);
                }
            }

            if let Some(access_time) = most_recent_access_time {
                dir_access_times.insert(sub_dir, access_time);
            }
        }
    }

    let mut sorted_dir_access_times: Vec<(PathBuf, FileTime)> =
        dir_access_times.into_iter().collect();
    sorted_dir_access_times.sort_unstable_by_key(|(_, accessed_time)| *accessed_time);
    sorted_dir_access_times.reverse();
    let dirs_to_remove = sorted_dir_access_times.split_off(num_to_keep);

    for (path, _) in dirs_to_remove {
        println!("Removing: {:?}", path);
        if let Err(err) = std::fs::remove_dir_all(path.clone()) {
            println!("Failed to remove {:?} due to: {:?}", path, err);
        }
    }

    Ok(())
}
