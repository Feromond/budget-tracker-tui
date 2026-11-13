use data_encoding::{BASE64, HEXLOWER};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const RELEASES_PAGE: &str = "https://github.com/Feromond/budget-tracker-tui/releases";
const MANIFEST_URL: &str =
    "https://github.com/Feromond/budget-tracker-tui/releases/latest/download/latest.json";
const DOWNLOAD_PREFIX: &str = "https://github.com/Feromond/budget-tracker-tui/releases/download/";
const BREW_API: &str = "https://formulae.brew.sh/api/formula/budget-tracker.json";
// Limit the response to fit `MAX_MANIFEST`.
const CRATES_API: &str =
    "https://crates.io/api/v1/crates/budget-tracker-tui?include=default_version";
const USER_AGENT: &str = concat!(
    "budget-tracker/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Feromond/budget-tracker-tui)"
);

// Must match CI's UPDATE_SIGNING_KEY; installed apps trust this key.
const PUBLIC_KEY_HEX: &str = "c010a4a4ca51d493ddff59ac828fc1221d730a5867f406f6865c0f9d86a9951d";
const CONTEXT: &[u8] = b"budget-tracker/update/v1";
const MAX_MANIFEST: u64 = 64 * 1024;
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

const TARGET: &str = env!("BUILD_TARGET");
const BIN_FILE: &str = if cfg!(windows) {
    "budget-tracker.exe"
} else {
    "budget-tracker"
};
const CRATE_NAME: &str = "budget-tracker-tui";
const BREW_FORMULA: &str = "budget-tracker";
const STAGING_PREFIX: &str = ".budget-tracker-update-";
// Allow an hour for cleanup; downloads time out after 10 minutes.
const STALE_STAGING: Duration = Duration::from_secs(60 * 60);
#[cfg(windows)]
const OLD_EXE: &str = "budget-tracker.old.exe";

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub struct Update {
    pub version: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub enum Action {
    Replace { exe: PathBuf, asset: Asset },
    Installer { asset: Asset },
    Brew { prefix: PathBuf },
    Cargo { root: PathBuf },
    Browser,
}

impl Action {
    pub fn in_app(&self) -> bool {
        matches!(self, Action::Replace { .. } | Action::Installer { .. })
    }

    pub fn download_size(&self) -> u64 {
        match self {
            Action::Replace { asset, .. } | Action::Installer { asset } => asset.size,
            _ => 0,
        }
    }

    pub fn command(&self) -> Option<String> {
        match self {
            Action::Brew { .. } => Some(format!("brew upgrade {BREW_FORMULA}")),
            Action::Cargo { .. } => Some(format!("cargo install {CRATE_NAME} --locked")),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum AfterExit {
    Restart(PathBuf),
    RunInstaller { path: PathBuf, asset: Asset },
    Upgrade { action: Action, version: String },
}

pub enum Progress {
    Downloading { received: u64, total: u64 },
    Finished(Result<AfterExit>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    url: String,
    size: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    files: BTreeMap<String, Asset>,
}

enum Install {
    Standalone(PathBuf),
    WindowsSetup,
    Homebrew(PathBuf),
    Cargo(PathBuf),
    Manual,
}

/// Blocks on the network; call it off the UI thread.
pub fn check() -> Option<Update> {
    let agent = agent(Duration::from_secs(5));
    let (version, action) = match detect_install() {
        Install::Homebrew(prefix) => {
            #[derive(Deserialize)]
            struct Formula {
                versions: Versions,
            }
            #[derive(Deserialize)]
            struct Versions {
                stable: String,
            }
            let formula: Formula =
                serde_json::from_slice(&get(&agent, BREW_API, MAX_MANIFEST)?).ok()?;
            (formula.versions.stable, Action::Brew { prefix })
        }
        Install::Cargo(root) => {
            #[derive(Deserialize)]
            struct Crate {
                #[serde(rename = "crate")]
                krate: Info,
            }
            #[derive(Deserialize)]
            struct Info {
                default_version: String,
            }
            let info: Crate =
                serde_json::from_slice(&get(&agent, CRATES_API, MAX_MANIFEST)?).ok()?;
            (info.krate.default_version, Action::Cargo { root })
        }
        install => {
            let manifest = fetch_manifest(&agent)?;
            let asset = |key: &str| {
                manifest
                    .files
                    .get(key)
                    .filter(|a| a.url.starts_with(DOWNLOAD_PREFIX) && a.size <= MAX_DOWNLOAD)
                    .cloned()
            };
            let action = match install {
                Install::Standalone(exe) => {
                    asset(TARGET).map(|asset| Action::Replace { exe, asset })
                }
                Install::WindowsSetup => {
                    asset(&format!("{TARGET}-setup")).map(|asset| Action::Installer { asset })
                }
                _ => None,
            };
            (manifest.version, action.unwrap_or(Action::Browser))
        }
    };
    is_newer(&version).then_some(Update { version, action })
}

fn is_newer(version: &str) -> bool {
    match (
        semver::Version::parse(version.trim_start_matches('v')),
        semver::Version::parse(env!("CARGO_PKG_VERSION")),
    ) {
        (Ok(remote), Ok(current)) => remote > current,
        _ => false,
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(timeout.min(Duration::from_secs(10))))
        .timeout_global(Some(timeout))
        .user_agent(USER_AGENT)
        .build()
        .new_agent()
}

fn get(agent: &ureq::Agent, url: &str, limit: u64) -> Option<Vec<u8>> {
    let response = agent.get(url).call().ok()?;
    response
        .into_body()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .ok()
}

fn fetch_manifest(agent: &ureq::Agent) -> Option<Manifest> {
    let bytes = get(agent, MANIFEST_URL, MAX_MANIFEST)?;
    let signature = get(agent, &format!("{MANIFEST_URL}.sig"), 1024)?;
    if !verify(&bytes, std::str::from_utf8(&signature).ok()?) {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}

fn verify(manifest: &[u8], signature: &str) -> bool {
    let key = HEXLOWER
        .decode(PUBLIC_KEY_HEX.as_bytes())
        .ok()
        .and_then(|k| <[u8; 32]>::try_from(k).ok());
    let signature = BASE64
        .decode(signature.trim().as_bytes())
        .ok()
        .and_then(|s| <[u8; 64]>::try_from(s).ok());
    let (Some(key), Some(signature)) = (key, signature) else {
        return false;
    };
    let mut message = Vec::with_capacity(CONTEXT.len() + 1 + manifest.len());
    message.extend_from_slice(CONTEXT);
    message.push(0);
    message.extend_from_slice(manifest);
    VerifyingKey::from_bytes(&key).is_ok_and(|key| {
        key.verify_strict(&message, &Signature::from_bytes(&signature))
            .is_ok()
    })
}

fn detect_install() -> Install {
    if cfg!(debug_assertions) {
        return Install::Manual;
    }
    let Ok(exe) = std::env::current_exe() else {
        return Install::Manual;
    };
    // Windows' canonical form is a \\?\ path, which some tools choke on.
    let exe = if cfg!(windows) {
        exe
    } else {
        exe.canonicalize().unwrap_or(exe)
    };
    let Some(dir) = exe.parent() else {
        return Install::Manual;
    };
    // unins000.exe is the old Inno Setup installer; the new setup migrates it.
    if cfg!(windows) && (dir.join("uninstall.exe").is_file() || dir.join("unins000.exe").is_file())
    {
        return Install::WindowsSetup;
    }
    if let Some(prefix) = homebrew_prefix(&exe) {
        return Install::Homebrew(prefix);
    }
    if let Some(root) = cargo_root(dir) {
        return Install::Cargo(root);
    }
    let system_managed = ["/usr/bin", "/usr/sbin", "/bin", "/nix", "/snap", "/opt"]
        .iter()
        .any(|p| exe.starts_with(p));
    if !system_managed && writable(dir) {
        Install::Standalone(exe)
    } else {
        Install::Manual
    }
}

/// `<prefix>/Cellar/budget-tracker/<version>/bin/budget-tracker` -> `<prefix>`.
fn homebrew_prefix(exe: &Path) -> Option<PathBuf> {
    let keg = exe.parent()?.ancestors().find(|p| {
        p.file_name().is_some_and(|n| n == BREW_FORMULA)
            && p.parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == "Cellar")
    })?;
    let prefix = keg.parent()?.parent()?;
    prefix
        .join("bin/brew")
        .is_file()
        .then(|| prefix.to_path_buf())
}

fn cargo_root(bin_dir: &Path) -> Option<PathBuf> {
    if bin_dir.file_name()? != "bin" {
        return None;
    }
    let root = bin_dir.parent()?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".crates2.json")).ok()?).ok()?;
    // Keys look like "budget-tracker-tui 1.7.0 (registry+https://...)".
    let ours = manifest.get("installs")?.as_object()?.keys().any(|key| {
        key.split(' ')
            .next()
            .is_some_and(|name| name.replace('_', "-") == CRATE_NAME)
    });
    ours.then(|| root.to_path_buf())
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".budget-tracker-probe-{}", std::process::id()));
    let ok = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    if ok {
        let _ = fs::remove_file(&probe);
    }
    ok
}

// Keep each instance's download separate.
fn staging_dir(parent: &Path) -> PathBuf {
    parent.join(format!("{STAGING_PREFIX}{}", std::process::id()))
}

pub fn clean_up() {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        return;
    };
    remove_stale_staging(&dir);
    #[cfg(windows)]
    {
        // Fails harmlessly while the instance that renamed it is still running.
        let _ = fs::remove_file(dir.join(OLD_EXE));
        remove_stale_staging(&std::env::temp_dir());
    }
}

fn remove_stale_staging(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > STALE_STAGING);
        let ours = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(STAGING_PREFIX));
        if ours && stale && entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Blocks; `progress` reports (bytes received, total). Cancellation stops before replacement.
pub fn install(
    action: &Action,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<AfterExit> {
    match action {
        Action::Replace { exe, asset } => {
            replace(exe, asset, cancel, &mut progress)?;
            Ok(AfterExit::Restart(exe.clone()))
        }
        Action::Installer { asset } => {
            let dir = staging_dir(&std::env::temp_dir());
            let _ = fs::remove_dir_all(&dir);
            let path = download(asset, &dir, cancel, &mut progress)?;
            Ok(AfterExit::RunInstaller {
                path,
                asset: asset.clone(),
            })
        }
        _ => Err("this install can't update itself".into()),
    }
}

const CANCELLED: &str = "the update was cancelled";

fn expected_hash(asset: &Asset) -> Result<Vec<u8>> {
    HEXLOWER
        .decode(asset.sha256.to_ascii_lowercase().as_bytes())
        .map_err(|_| "the update information is malformed".into())
}

fn download(
    asset: &Asset,
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<PathBuf> {
    let expected = expected_hash(asset)?;
    let name = asset
        .url
        .rsplit('/')
        .next()
        .filter(|name| {
            !name.is_empty()
                && !name.starts_with('.')
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        })
        .ok_or("the update information is malformed")?;
    fs::create_dir_all(dir).map_err(io_error)?;
    let path = dir.join(name);
    let partial = dir.join(format!("{name}.part"));

    let response = agent(Duration::from_secs(10 * 60))
        .get(&asset.url)
        .call()
        .map_err(|e| format!("couldn't download the update: {e}"))?;
    let mut body = response.into_body().into_reader();
    let _ = fs::remove_file(&partial);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    let result = loop {
        if cancel.load(Ordering::Relaxed) {
            break Err(CANCELLED.into());
        }
        let n = match body.read(&mut buffer) {
            Ok(0) => break Ok(()),
            Ok(n) => n,
            Err(e) => break Err(format!("couldn't download the update: {e}")),
        };
        received += n as u64;
        if received > asset.size {
            break Err("the download doesn't match the update".into());
        }
        hasher.update(&buffer[..n]);
        if let Err(e) = file.write_all(&buffer[..n]) {
            break Err(io_error(e));
        }
        progress(received, asset.size);
    };
    let result = result
        .and_then(|_| file.sync_all().map_err(io_error))
        .and_then(|_| {
            if received == asset.size && hasher.finalize().as_slice() == expected.as_slice() {
                Ok(())
            } else {
                Err("the download doesn't match the update".into())
            }
        });
    drop(file);
    if let Err(e) = result {
        let _ = fs::remove_file(&partial);
        return Err(e);
    }
    fs::rename(&partial, &path).map_err(io_error)?;
    Ok(path)
}

// The staged file may have changed since download.
fn verify_file(path: &Path, asset: &Asset) -> Result<()> {
    let expected = expected_hash(asset)?;
    let mut file = fs::File::open(path).map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(io_error)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        hasher.update(&buffer[..n]);
    }
    if size == asset.size && hasher.finalize().as_slice() == expected.as_slice() {
        Ok(())
    } else {
        Err("the download doesn't match the update".into())
    }
}

fn replace(
    exe: &Path,
    asset: &Asset,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(u64, u64),
) -> Result<()> {
    let dir = exe.parent().ok_or("can't find the app's folder")?;
    // Staged next to the binary so the final rename stays on one filesystem.
    let staging = staging_dir(dir);
    let _ = fs::remove_dir_all(&staging);
    let archive = download(asset, &staging, cancel, progress)?;

    let tar = if cfg!(windows) {
        // System32's bsdtar reads zips; a GNU tar earlier on PATH wouldn't.
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32\\tar.exe")
    } else {
        PathBuf::from("tar")
    };
    let status = Command::new(tar)
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("couldn't unpack the update: {e}"))?;
    if !status.success() {
        return Err("couldn't unpack the update".into());
    }
    let new = fs::read_dir(&staging)
        .map_err(io_error)?
        .filter_map(|e| e.ok().map(|e| e.path().join(BIN_FILE)))
        .find(|p| p.is_file())
        .ok_or("the update has no app in it")?;
    if cancel.load(Ordering::Relaxed) {
        let _ = fs::remove_dir_all(&staging);
        return Err(CANCELLED.into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new, fs::Permissions::from_mode(0o755)).map_err(io_error)?;
    }
    // A running exe can't be overwritten on Windows, but it can be renamed.
    #[cfg(windows)]
    {
        let old = dir.join(OLD_EXE);
        let _ = fs::remove_file(&old);
        fs::rename(exe, &old).map_err(io_error)?;
        if let Err(e) = fs::rename(&new, exe) {
            let _ = fs::rename(&old, exe);
            return Err(io_error(e));
        }
    }
    #[cfg(not(windows))]
    fs::rename(&new, exe).map_err(io_error)?;

    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

fn io_error(e: std::io::Error) -> String {
    format!("couldn't install the update: {e}")
}

/// Call after restoring the terminal.
pub fn run_after_exit(after: AfterExit) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match after {
        AfterExit::Restart(exe) => restart(&exe),
        AfterExit::RunInstaller { path, asset } => {
            verify_file(&path, &asset)?;
            // /P: passive, /R: reopen the app after, /NS: don't recreate deleted shortcuts.
            Command::new(&path)
                .args(["/P", "/R", "/NS"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            println!(
                "Installing the update. Budget Tracker reopens in a new window when it's done."
            );
            Ok(())
        }
        AfterExit::Upgrade { action, version } => upgrade(&action, &version),
    }
}

fn restart(exe: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(Command::new(exe).exec().into())
    }
    #[cfg(not(unix))]
    {
        let status = Command::new(exe).status()?;
        std::process::exit(status.code().unwrap_or(0));
    }
}

fn upgrade(action: &Action, version: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let shown = action.command().unwrap_or_default();
    let (mut command, relaunch) = match action {
        Action::Brew { prefix } => {
            let mut command = Command::new(prefix.join("bin/brew"));
            command
                .args(["upgrade", BREW_FORMULA])
                // Refresh brew's index so it sees the new release.
                .env("HOMEBREW_AUTO_UPDATE_SECS", "0");
            // The old Cellar path is gone; bin/ links to the new version.
            (command, prefix.join("bin").join(BIN_FILE))
        }
        Action::Cargo { root } => {
            let bin = root.join("bin");
            let cargo = bin.join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
            let mut command = Command::new(if cargo.is_file() {
                cargo.into_os_string()
            } else {
                "cargo".into()
            });
            command
                .args(["install", CRATE_NAME, "--locked", "--root"])
                .arg(root);
            (command, bin.join(BIN_FILE))
        }
        _ => return Ok(()),
    };

    println!("Updating Budget Tracker to {version}: {shown}\n");
    #[cfg(windows)]
    let parked = {
        let old = relaunch.with_file_name(OLD_EXE);
        let _ = fs::remove_file(&old);
        fs::rename(&relaunch, &old).ok().map(|_| old)
    };
    let status = command.status();
    let succeeded = matches!(&status, Ok(s) if s.success());
    #[cfg(windows)]
    if let Some(old) = parked
        && !relaunch.is_file()
    {
        let _ = fs::rename(old, &relaunch);
    }
    if !succeeded {
        let reason = match status {
            Err(e) => format!(" ({e})"),
            Ok(_) => String::new(),
        };
        eprintln!("\nThe update didn't finish{reason}. You can run it yourself: {shown}");
    } else {
        let installed = Command::new(&relaunch)
            .arg("--version")
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        if installed.split_whitespace().any(|word| word == version) {
            return restart(&relaunch);
        }
        println!("\nBudget Tracker {version} isn't available there yet. Try again later.");
    }
    if !relaunch.is_file() {
        std::process::exit(1);
    }
    println!("Press Enter to reopen Budget Tracker.");
    let _ = std::io::stdin().read_line(&mut String::new());
    restart(&relaunch)
}
