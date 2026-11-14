use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub(crate) struct Place {
    pub(crate) label: String,
    pub(crate) path: Option<PathBuf>,
}

impl Place {
    fn header(label: &str) -> Self {
        Self {
            label: label.to_string(),
            path: None,
        }
    }
}

pub(crate) fn discover() -> Vec<Place> {
    let mut favorites: Vec<(String, PathBuf)> = Vec::new();
    let home = dirs::home_dir();
    if let Some(home) = &home {
        favorites.push(("Home".to_string(), home.clone()));
    }
    for (label, dir) in [
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Downloads", dirs::download_dir()),
    ] {
        if let Some(dir) = dir {
            favorites.push((label.to_string(), dir));
        }
    }
    if let Some(home) = &home {
        favorites.extend(cloud_folders(home));
    }

    let mut places = vec![Place::header("Folders")];
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut push = |places: &mut Vec<Place>, label: String, path: PathBuf| {
        if path.is_dir() && !seen.contains(&path) {
            seen.push(path.clone());
            places.push(Place {
                label,
                path: Some(path),
            });
        }
    };
    for (label, path) in favorites {
        push(&mut places, label, path);
    }
    let drives = cached_drives();
    if !drives.is_empty() {
        places.push(Place::header(""));
        places.push(Place::header("Drives"));
        for (label, path) in drives {
            push(&mut places, label, path);
        }
    }
    places
}

fn cloud_folders(home: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        found.push((
            "iCloud Drive".to_string(),
            home.join("Library/Mobile Documents/com~apple~CloudDocs"),
        ));
        found.extend(
            subfolders(&home.join("Library/CloudStorage")).map(|(name, path)| {
                let label = match name.split_once('-') {
                    Some((provider, account)) => {
                        format!("{} ({})", provider_label(provider), account)
                    }
                    None => provider_label(&name),
                };
                (label, path)
            }),
        );
    }
    for name in ["iCloudDrive", "Dropbox", "Google Drive", "Nextcloud"] {
        found.push((
            if name == "iCloudDrive" {
                "iCloud Drive".to_string()
            } else {
                name.to_string()
            },
            home.join(name),
        ));
    }
    // OneDrive folder names can include a company name.
    found.extend(subfolders(home).filter(|(name, _)| name.starts_with("OneDrive")));
    found
}

fn provider_label(provider: &str) -> String {
    match provider {
        "GoogleDrive" => "Google Drive".to_string(),
        other => other.to_string(),
    }
}

// Scan once per session; disconnected network drives can stall.
fn cached_drives() -> Vec<(String, PathBuf)> {
    static DRIVES: OnceLock<Vec<(String, PathBuf)>> = OnceLock::new();
    DRIVES.get_or_init(drives).clone()
}

fn drives() -> Vec<(String, PathBuf)> {
    if cfg!(windows) {
        // Skip floppy drives A: and B: to avoid slow checks.
        return (b'C'..=b'Z')
            .map(|letter| format!("{}:\\", letter as char))
            .filter(|root| Path::new(root).is_dir())
            .map(|root| (root.clone(), PathBuf::from(root)))
            .collect();
    }
    if cfg!(target_os = "macos") {
        return subfolders(Path::new("/Volumes")).collect();
    }
    let mut drives = vec![("/".to_string(), PathBuf::from("/"))];
    let user = std::env::var("USER").unwrap_or_default();
    for root in [
        PathBuf::from("/media").join(&user),
        PathBuf::from("/run/media").join(&user),
        PathBuf::from("/mnt"),
    ] {
        drives.extend(subfolders(&root));
    }
    drives
}

fn subfolders(dir: &Path) -> impl Iterator<Item = (String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().to_string(),
                entry.path(),
            )
        })
        .filter(|(name, path)| !name.starts_with('.') && path.is_dir())
        .collect();
    found.sort_by_key(|(name, _)| name.to_lowercase());
    found.into_iter()
}
