use std::{
    cmp::max,
    fs,
    io::{Cursor, Read, Write},
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{self, AtomicBool, AtomicU64, AtomicUsize},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use arc_swap::ArcSwap;
use fnv::FnvHashMap;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use size::Size;
use thread_priority::ThreadPriority;

use super::{
    gui::SimpleYesNoDialog,
    hachimi::LocalizedData,
    http::{self, ureq_config, AsyncRequest},
    utils, Error, Gui, Hachimi,
};
use crate::core::game::Region;
use once_cell::sync::Lazy;

#[derive(Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub index: String,
    pub short_desc: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub region: Region,
    #[serde(default)]
    pub index_mod: Option<String>,
}

impl RepoInfo {
    pub fn is_recommended(&self, current_lang_str: &str) -> bool {
        let Some(repo_tag) = self.language.as_deref() else {
            return false;
        };
        let repo_tag = repo_tag.to_lowercase();
        let target = current_lang_str.to_lowercase();

        if repo_tag == target || repo_tag.starts_with(&target) {
            return true;
        }

        let sys = sys_locale::get_locale()
            .as_deref()
            .unwrap_or("en")
            .to_lowercase();
        repo_tag.starts_with(&sys) || sys.starts_with(&repo_tag)
    }
}

// Repo registry (ported from kairusds/Hachimi-Edge, minus the info.json fluff).
// Persisted as <data>/.tl_repos — maps a numeric id to the repo index URL.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct RepoList {
    pub repos: Vec<RepoEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoEntry {
    pub id: u32,
    pub index: String,
}

impl RepoList {
    pub fn load(path: &Path) -> Result<Self, Error> {
        if path.exists() {
            let data = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        utils::write_json_file(self, path)
    }

    pub fn next_id(&self) -> u32 {
        self.repos
            .iter()
            .map(|r| r.id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(1)
    }

    pub fn add(&mut self, index: String) -> u32 {
        let id = self.next_id();
        self.repos.push(RepoEntry { id, index });
        id
    }

    pub fn find_by_index(&self, index: &str) -> Option<u32> {
        self.repos.iter().find(|r| r.index == index).map(|r| r.id)
    }

    pub fn find_by_id(&self, id: u32) -> Option<&str> {
        self.repos
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.index.as_str())
    }
}

pub fn new_meta_index_request() -> AsyncRequest<Vec<RepoInfo>> {
    let meta_index_url = &Hachimi::instance().config.load().meta_index_url;

    let req = ureq::http::Request::builder()
        .uri(meta_index_url)
        .method("GET")
        .body(ureq::Body::builder().reader(std::io::empty()))
        .expect("Failed to build meta index request");

    AsyncRequest::with_json_response(req)
}

#[derive(Deserialize)]
struct RepoIndex {
    base_url: String,
    zip_url: String,
    zip_dir: String,
    files: Vec<RepoFile>,
}

#[derive(Deserialize, Clone)]
struct RepoFile {
    path: String,
    hash: String,
    size: usize,
}

impl RepoFile {
    fn get_fs_path(&self, root_dir: &Path) -> PathBuf {
        // Modern Windows versions support forward slashes anyways but it doesn't hurt to do something so trivial
        #[cfg(target_os = "windows")]
        return root_dir.join(&self.path.replace("/", "\\"));

        #[cfg(not(target_os = "windows"))]
        return root_dir.join(&self.path);
    }
    fn verify_integrity(&self, full_path: &Path) -> bool {
        let Ok(mut file) = fs::File::open(full_path) else {
            return false;
        };
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0u8; 8192];

        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        hasher.finalize().to_hex().as_str() == self.hash
    }
}

#[derive(Clone)]
struct UpdateInfo {
    base_url: String,
    zip_url: String,
    zip_dir: String,
    files: Vec<RepoFile>, // only contains files needed for update
    is_new_repo: bool,
    pedantic: bool, // whether this was a pedantic check (don't cascade to addon)
    cached_files: FnvHashMap<String, String>, // from repo cache
    size: usize,
    // New fields for better user communication, idk why it complains about these never being read
    #[allow(dead_code)]
    update_size: usize, // Size of changed files only
    #[allow(dead_code)]
    total_size: usize, // Total size of all files (for ZIP downloads)
    will_use_zip: bool,   // Whether ZIP download will be used
    modifies_atlas: bool, // Whether file updates include atlases
    index_etag: Option<String>,
}

#[derive(Default, Clone)]
pub struct UpdateProgress {
    pub current: usize,
    pub total: usize,
}

impl UpdateProgress {
    pub fn new(current: usize, total: usize) -> UpdateProgress {
        UpdateProgress { current, total }
    }
}

const REPO_CACHE_FILENAME: &str = ".tl_repo_cache";
const REPO_CACHE_MOD_FILENAME: &str = ".tl_repo_cache_mod";
const REPO_EXCLUDES_FILENAME: &str = ".tl_repo_excludes";

/// True when `path` matches an exclude entry. Matches exact paths and whole
/// directories: an entry `story` also excludes `story/...` (anything below it),
/// while an entry with a trailing slash (`story/`) only matches the folder
/// itself. Mirrors upstream kairusds directory-matching semantics.
fn is_excluded(excludes: &HashSet<String>, path: &str) -> bool {
    excludes.iter().any(|exc| {
        if path == *exc {
            return true;
        }
        let exc_dir = exc.trim_end_matches('/');
        !exc_dir.is_empty() && path.starts_with(&format!("{}/", exc_dir))
    })
}
#[derive(Serialize, Deserialize, Default)]
struct RepoCache {
    base_url: String,
    #[serde(default)]
    index_etag: Option<String>,
    files: FnvHashMap<String, String>, // path: hash
}

#[derive(Clone)]
struct ModUpdateInfo {
    base_url: String,
    zip_url: String,
    zip_dir: String,
    files: Vec<RepoFile>,
    cached_files: FnvHashMap<String, String>,
    size: usize,
    #[allow(dead_code)]
    update_size: usize,
    #[allow(dead_code)]
    total_size: usize,
    will_use_zip: bool,
}

#[derive(Default)]
pub struct Updater {
    update_check_mutex: Mutex<()>,
    run_mutex: Mutex<()>,
    new_update: ArcSwap<Option<UpdateInfo>>,
    progress: ArcSwap<Option<UpdateProgress>>,
    /// True during `run_internal` (actual file download), false during the
    /// check/scan phase. Used by the GUI to show "Checking..." vs "Updating...".
    is_downloading: AtomicBool,
    skipped_etag: Mutex<Option<String>>,
    new_mod_update: ArcSwap<Option<ModUpdateInfo>>,
    mod_progress: ArcSwap<Option<UpdateProgress>>,
    last_progress_ms: AtomicU64,
    last_mod_progress_ms: AtomicU64,
}

const LOCALIZED_DATA_DIR: &str = "localized_data";
const CHUNK_SIZE: usize = 8192; // 8KiB

fn get_repo_cache_path(id: u32) -> PathBuf {
    Hachimi::instance().get_data_path(format!(".tl_repo_cache_{id}"))
}
static NUM_THREADS: Lazy<usize> = Lazy::new(|| {
    let parallelism = thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    max(1, parallelism / 2)
});

const INCREMENTAL_UPDATE_LIMIT_GITHUB: usize = 55;
const INCREMENTAL_UPDATE_LIMIT_GITLAB: usize = 250;
const INCREMENTAL_SIZE_RATIO_THRESHOLD: f64 = 0.8;
const ZIP_SIZE_WARNING_RATIO: f64 = 1.2; // Warn if ZIP is 1.2x larger than changes

const MIN_CHUNK_SIZE: u64 = 1024 * 1024 * 5;

struct DownloadJob {
    agent: ureq::Agent,
    hasher: blake3::Hasher,
    buffer: Vec<u8>,
}

impl DownloadJob {
    fn new(agent1: ureq::Agent) -> DownloadJob {
        DownloadJob {
            agent: agent1,
            hasher: blake3::Hasher::new(),
            buffer: vec![0u8; CHUNK_SIZE],
        }
    }
}

// 60fps time-based throttle for progress bar updates
fn store_progress(
    progress: &ArcSwap<Option<UpdateProgress>>,
    last_progress_ms: &AtomicU64,
    current: usize,
    total: usize,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let last = last_progress_ms.load(atomic::Ordering::Relaxed);
    if current == total || now.saturating_sub(last) >= 16 {
        last_progress_ms.store(now, atomic::Ordering::Relaxed);
        progress.store(Arc::new(Some(UpdateProgress::new(current, total))));
    }
}

/// RAII guard that ensures a temporary ZIP file is cleaned up, even if the function returns early via `?` or panics
struct ZipCleanupGuard<'a>(&'a Path);
impl Drop for ZipCleanupGuard<'_> {
    fn drop(&mut self) {
        if self.0.exists() {
            if let Err(e) = fs::remove_file(self.0) {
                error!("Failed to clean up temporary ZIP file '{}': {}", self.0.display(), e);
            }
        }
    }
}

fn check_available_disk_space(_path: &Path, required_bytes: u64) -> Result<(), Error> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;

        let wide: Vec<u16> = _path.as_os_str().encode_wide().chain(std::iter::once(0u16)).collect();
        let mut free_bytes: u64 = 0;

        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
                lpFreeBytesAvailableToCaller: *mut u64,
            ) -> i32;
        }

        let result = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        if result != 0 && free_bytes < required_bytes {
            return Err(Error::OutOfDiskSpace);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path_bytes = _path.as_os_str().as_bytes();
        if let Ok(c_path) = CString::new(path_bytes) {
            unsafe {
                let mut stat: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                    let free_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
                    if free_bytes < required_bytes {
                        return Err(Error::OutOfDiskSpace);
                    }
                }
            }
        }
    }

    Ok(())
}

impl Updater {
    fn normalize_zip_path(path: &str) -> String {
        path.trim_matches('/').replace('\\', "/")
    }

    fn resolve_zip_entry_repo_file<'a>(
        files_to_extract: &'a FnvHashMap<String, RepoFile>,
        zip_entry_name: &str,
        zip_dir: &str,
    ) -> Option<&'a RepoFile> {
        let normalized_entry = Self::normalize_zip_path(zip_entry_name);
        let normalized_zip_dir = Self::normalize_zip_path(zip_dir);

        let entry_candidates = {
            let entry_without_zip_dir = if normalized_zip_dir.is_empty() {
                normalized_entry.clone()
            } else {
                normalized_entry
                    .strip_prefix(&format!("{normalized_zip_dir}/"))
                    .unwrap_or(&normalized_entry)
                    .to_string()
            };
            let entry_without_localized_data = normalized_entry
                .strip_prefix("localized_data/")
                .unwrap_or(&normalized_entry)
                .to_string();
            let entry_without_megamtl = normalized_entry
                .strip_prefix("megamtl/localized_data/")
                .unwrap_or(&normalized_entry)
                .to_string();

            [
                normalized_entry.clone(),
                entry_without_zip_dir,
                entry_without_localized_data,
                entry_without_megamtl,
            ]
        };

        for (expected_path, repo_file) in files_to_extract {
            let normalized_expected = Self::normalize_zip_path(expected_path);
            let normalized_repo_path = Self::normalize_zip_path(&repo_file.path);
            let repo_candidates = [
                normalized_expected.clone(),
                normalized_repo_path.clone(),
                format!("{normalized_zip_dir}/{normalized_repo_path}"),
                format!("localized_data/{normalized_repo_path}"),
                format!("megamtl/localized_data/{normalized_repo_path}"),
            ];

            if entry_candidates.iter().any(|entry| {
                repo_candidates.iter().any(|candidate| candidate == entry)
            }) {
                return Some(repo_file);
            }
        }

        None
    }

    fn populate_existing_mod_files(cache_files: &mut FnvHashMap<String, String>, files: &[RepoFile], ld_dir: &Path) {
        for file in files {
            let path = ld_dir.join(&file.path);
            if path.is_file()
                && fs::metadata(&path)
                    .map(|m| m.len() as usize == file.size)
                    .unwrap_or(false)
                && file.verify_integrity(&path)
            {
                cache_files.insert(file.path.clone(), file.hash.clone());
            }
        }
    }

    fn cleanup_partial_file(path: &Path) {
        if path.exists() {
            if let Err(e) = fs::remove_file(path) {
                warn!("Failed to remove partial file '{}': {}", path.display(), e);
            }
        }
    }

    fn log_corrupted_download(path: &Path, url: &str, expected_hash: &str, actual_hash: &str) {
        error!(
            "Corrupted download detected for '{}' from '{}': expected {} got {}",
            path.display(),
            url,
            expected_hash,
            actual_hash
        );
    }

    pub fn skip_update(&self, etag: Option<String>) {
        *self.skipped_etag.lock().unwrap_or_else(|e| e.into_inner()) = etag;
    }

    pub fn has_pending_update(&self) -> bool {
        self.new_update.load().is_some()
    }

    pub fn clear_pending_update(&self) {
        self.new_update.store(Arc::new(None));
    }

    pub fn has_pending_mod_update(&self) -> bool {
        self.new_mod_update.load().is_some()
    }

    pub fn clear_pending_mod_update(&self) {
        self.new_mod_update.store(Arc::new(None));
    }

    pub fn check_for_updates(self: Arc<Self>, pedantic: bool, silent: bool) {
        std::thread::spawn(move || {
            if let Err(e) = self.check_for_updates_internal(pedantic, pedantic, silent) {
                if let Some(mutex) = Gui::instance() {
                    if !silent {
                        mutex.lock().unwrap_or_else(|e| e.into_inner()).show_notification(&format!("{}", e));
                    }
                }
                info!("{}", e);
            }
        });
    }

    pub fn check_for_mod_updates_only(self: Arc<Self>, pedantic: bool, silent: bool) {
        std::thread::spawn(move || {
            if let Err(e) = self.check_for_mod_updates_only_internal(pedantic, silent) {
                if let Some(mutex) = Gui::instance() {
                    if !silent {
                        mutex.lock().unwrap_or_else(|e| e.into_inner()).show_notification(&format!("{}", e));
                    }
                }
                info!("{}", e);
            }
        });
    }

    fn check_for_mod_updates_only_internal(&self, pedantic: bool, silent: bool) -> Result<(), Error> {
        let Ok(_guard) = self.update_check_mutex.try_lock() else {
            return Ok(());
        };

        if self.has_pending_update() || self.has_pending_mod_update() {
            return Ok(());
        }

        if self.is_downloading.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let Some(mod_index_url) = &config.translation_repo_index_mod else {
            return Ok(());
        };

        if config.disable_mod_downloads {
            return Ok(());
        }

        let ld_dir_path = config
            .localized_data_dir
            .as_ref()
            .map(|p| hachimi.get_data_path(p));

        if !silent {
            if let Some(mutex) = Gui::instance() {
                mutex
                    .lock()
                    .unwrap()
                    .show_notification(&t!("notification.checking_for_addon_updates"));
            }
        }

        let found = self.check_for_mod_updates(mod_index_url, pedantic, silent, &config, &ld_dir_path)?;
        if !found && !silent {
            if let Some(mutex) = Gui::instance() {
                mutex
                    .lock()
                    .unwrap()
                    .show_notification(&t!("notification.no_addon_updates"));
            }
        }
        Ok(())
    }

    fn is_github_hosted(url: &str) -> bool {
        url.contains("github.com")
            || url.contains("githubusercontent.com")
            || url.contains("github.io")
    }

    fn is_gitlab_hosted(url: &str) -> bool {
        url.contains("gitlab.com") || url.contains("gitlab.io")
    }

    fn should_use_zip_download(
        file_count: usize,
        update_size: usize,
        total_size: usize,
        base_url: &str,
    ) -> bool {
        // if it's on GitHub and the update has > 55 files, use ZIP to avoid 403 errors
        if Self::is_github_hosted(base_url) && file_count > INCREMENTAL_UPDATE_LIMIT_GITHUB {
            return true;
        }

        // for GitLab, 250 file limit is a safe safe buffer below the raw endpoint cap of 300
        if Self::is_gitlab_hosted(base_url) && file_count > INCREMENTAL_UPDATE_LIMIT_GITLAB {
            return true;
        }

        // as long as the update is less than 80% of the total size of the repo, keep it incremental
        if (update_size as f64) < (total_size as f64 * INCREMENTAL_SIZE_RATIO_THRESHOLD) {
            return false;
        }

        // if the update >80% of the repo size, just grab the ZIP
        true
    }

    fn check_for_updates_internal(&self, pedantic_main: bool, pedantic_mod: bool, silent: bool) -> Result<(), Error> {
        // Prevent multiple update checks running at the same time
        let Ok(_guard) = self.update_check_mutex.try_lock() else {
            return Ok(());
        };

        if self.has_pending_update() || self.has_pending_mod_update() {
            return Ok(());
        }

        if self.is_downloading.load(atomic::Ordering::Relaxed) {
            return Ok(());
        }

        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let Some(index_url) = &config.translation_repo_index else {
            return Ok(());
        };

        // Ensure the repo is registered so the active dir + per-repo cache are keyed by id.
        if config.selected_tl_repo_id.is_none() {
            let id = {
                let mut manager = hachimi.tl_repo_manager.lock().unwrap();
                let repos_path = hachimi.get_data_path(".tl_repos");
                match manager.find_by_index(index_url) {
                    Some(existing) => existing,
                    None => {
                        let new_id = manager.add(index_url.clone());
                        if let Err(e) = manager.save(&repos_path) {
                            warn!("Failed to save .tl_repos: {e}");
                        }
                        new_id
                    }
                }
            };
            let mut new_config = (**config).clone();
            new_config.selected_tl_repo_id = Some(id);
            hachimi.save_and_reload_config(new_config)?;
        }
        let config = hachimi.config.load(); // in case repo id was migrated
        let ld_dir_path = hachimi.get_active_tl_dir().or_else(|| {
            config
                .localized_data_dir
                .as_ref()
                .map(|p| hachimi.get_data_path(p))
        });

        if !silent {
            if let Some(mutex) = Gui::instance() {
                // Non-persistent: auto-dismisses after 4s like all other snackbars.
                mutex
                    .lock()
                    .unwrap()
                    .show_notification(&t!("notification.checking_for_tl_updates"));
            }
        }

        let cache_path = config
            .selected_tl_repo_id
            .map(get_repo_cache_path)
            .unwrap_or_else(|| hachimi.get_data_path(REPO_CACHE_FILENAME));
        let repo_cache = if fs::metadata(&cache_path).is_ok() {
            let json = fs::read_to_string(&cache_path)?;
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            RepoCache::default()
        };

        let excludes_path = hachimi.get_data_path(REPO_EXCLUDES_FILENAME);
        let excludes: HashSet<String> = if excludes_path.exists() {
            fs::read_to_string(&excludes_path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.trim().replace("\\", "/"))
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            HashSet::new()
        };

        let mod_managed_paths: HashSet<String> = if !config.disable_mod_downloads && config.translation_repo_index_mod.is_some() {
            let mod_cache_path = hachimi.get_data_path(REPO_CACHE_MOD_FILENAME);
            if fs::metadata(&mod_cache_path).is_ok() {
                let json = fs::read_to_string(&mod_cache_path).unwrap_or_default();
                let mod_cache: RepoCache = serde_json::from_str(&json).unwrap_or_default();
                mod_cache.files.keys().cloned().collect()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };

        let mut new_etag: Option<String> = None;

        // Conditional GET: send If-None-Match with the best known ETag so the server
        // can reply 304 Not Modified when nothing has changed, avoiding a full index
        // download on every periodic check. Pedantic runs bypass the ETag entirely —
        // the user explicitly asked for a full fetch + integrity re-scan.
        let etag_for_request: Option<String> = if !pedantic_main {
            repo_cache.index_etag.clone()
                .or_else(|| self.skipped_etag.lock().unwrap_or_else(|e| e.into_inner()).clone())
        } else {
            None
        };

        let mut request = ureq::agent().get(index_url);
        if let Some(ref etag) = etag_for_request {
            request = request.header("If-None-Match", etag);
        }

        let index: RepoIndex = match request.call() {
            Ok(res) => {
                // 304 Not Modified — server honoured If-None-Match.
                // ureq 3 returns this as Ok rather than Err, so check explicitly.
                if res.status() == ureq::http::StatusCode::NOT_MODIFIED {
                    info!("Server returned 304 Not Modified. No translation updates available.");
                    if !silent {
                        if let Some(mutex) = Gui::instance() {
                            mutex.lock().unwrap_or_else(|e| e.into_inner())
                                .show_notification(&t!("notification.no_tl_updates"));
                        }
                    }
                    return Ok(());
                }

                if let Some(etag_val) = res.headers().get("ETag") {
                    if let Ok(etag_str) = etag_val.to_str() {
                        let etag_string = etag_str.to_string();

                        // User previously dismissed this exact version — skip.
                        if !pedantic_main {
                            if let Some(skipped) = &*self.skipped_etag.lock().unwrap_or_else(|e| e.into_inner()) {
                                if skipped == &etag_string {
                                    debug!("Server ETag matches skipped ETag. Ignoring update.");
                                    return Ok(());
                                }
                            }
                        }

                        // Fallback for servers that ignore If-None-Match: compare ETag manually.
                        if !pedantic_main {
                            if let Some(cached) = &repo_cache.index_etag {
                                if cached == &etag_string {
                                    debug!("Server ETag matches cached ETag (server may not support conditional requests). Continuing scan for addon-only updates.");
                                    // Don't exit — fall through to file scan in case addon has updates.
                                }
                            }
                        }

                        new_etag = Some(etag_string);
                    }
                }
                serde_json::from_reader(res.into_body().into_reader())?
            }
            Err(ureq::Error::StatusCode(code)) if code == ureq::http::StatusCode::NOT_MODIFIED => {
                // Kept as a safety net in case ureq behaviour changes in a future version.
                info!("Server returned 304 Not Modified. No translation updates available.");
                if !silent {
                    if let Some(mutex) = Gui::instance() {
                        mutex.lock().unwrap_or_else(|e| e.into_inner())
                            .show_notification(&t!("notification.no_tl_updates"));
                    }
                }
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        let is_new_repo = index.base_url != repo_cache.base_url;
        let mut modifies_atlas = false;
        let mut update_files: Vec<RepoFile> = Vec::new();
        let mut update_size: usize = 0;
        let mut total_size: usize = 0;

        let total_files = index.files.len().max(1);
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = true;
            }
        }

        for (i, file) in index.files.iter().enumerate() {
            if i % 50 == 0 {
                self.progress
                    .store(Arc::new(Some(UpdateProgress::new(i, total_files))));
            }

            if file.path.contains("..") || Path::new(&file.path).has_root() {
                warn!("File path '{}' sanitized", file.path);
                continue;
            }

            // Keep addon-managed files owned by the addon repo from being reprocessed by the main updater.
            if mod_managed_paths.contains(&file.path) {
                total_size += file.size;
                continue;
            }

            let path = ld_dir_path.as_ref().map(|p| p.join(&file.path));
            let exists = path.as_ref().map(|p| p.is_file()).unwrap_or(false);

            let updated = if is_new_repo {
                // redownload every single file because the directory will be deleted
                true
            } else if !pedantic_main && config.lazy_translation_updates {
                if let Some(hash) = repo_cache.files.get(&file.path) {
                    hash != &file.hash
                } else {
                    true
                }
            } else if let Some(hash) = repo_cache.files.get(&file.path) {
                if !pedantic_main && exists && is_excluded(&excludes, &file.path) {
                    false
                } else if let Some(path) = path {
                    // file doesn't exist -> download
                    if !exists {
                        true
                    } else {
                        if hash != &file.hash {
                            true // index hash changed -> update
                        } else if fs::metadata(&path)
                            .map(|m| m.len() as usize != file.size)
                            .unwrap_or(true)
                        {
                            true // size mismatch -> redownload
                        } else if pedantic_main {
                            !file.verify_integrity(&path)
                        } else {
                            false // everything matches -> skip
                        }
                    }
                } else {
                    true // path invalid -> download
                }
            } else {
                // file doesn't exist in cache at all -> download it
                true
            };

            if updated {
                update_files.push(file.clone());
                update_size += file.size;
                if file.path.contains("/atlas/") {
                    modifies_atlas = true;
                }
            }
            total_size += file.size;
        }

        self.progress.store(Arc::new(None));
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = false;
            }
        }

        if !update_files.is_empty() {
            // Determine download strategy
            let will_use_zip = Self::should_use_zip_download(
                update_files.len(),
                update_size,
                total_size,
                &index.base_url,
            );

            // Calculate actual download size
            let actual_download_size = if will_use_zip {
                total_size
            } else {
                update_size
            };

            // Store update info with all relevant sizes
            self.new_update.store(Arc::new(Some(UpdateInfo {
                is_new_repo,
                pedantic: pedantic_main,
                base_url: index.base_url,
                zip_url: index.zip_url,
                zip_dir: index.zip_dir,
                files: update_files,
                cached_files: repo_cache.files,
                size: actual_download_size,
                update_size,
                total_size,
                will_use_zip,
                modifies_atlas,
                index_etag: new_etag.clone(),
            })));

            if silent {
                Hachimi::instance().tl_updater.clone().run();
            } else if let Some(mutex) = Gui::instance() {
                // Determine the dialog message based on download strategy
                let dialog_message = if will_use_zip && update_size > 0 {
                    let size_ratio = total_size as f64 / update_size.max(1) as f64;

                    if size_ratio >= ZIP_SIZE_WARNING_RATIO {
                        // Warn user about larger ZIP download
                        debug!(
                            "ZIP download warning: changed={} MB, total={} MB, ratio={:.2}x",
                            update_size / (1024 * 1024),
                            total_size / (1024 * 1024),
                            size_ratio
                        );

                        t!(
                            "tl_update_dialog.content_zip_warning",
                            changed_size = Size::from_bytes(update_size),
                            download_size = Size::from_bytes(total_size)
                        )
                    } else {
                        // ZIP is being used but size difference is not significant
                        t!(
                            "tl_update_dialog.content",
                            size = Size::from_bytes(actual_download_size)
                        )
                    }
                } else {
                    // Incremental update or no warning needed
                    t!(
                        "tl_update_dialog.content",
                        size = Size::from_bytes(actual_download_size)
                    )
                };

                let updater = Hachimi::instance().tl_updater.clone();
                let etag_to_skip = new_etag.clone();

                mutex
                    .lock()
                    .unwrap()
                    .show_window(Box::new(SimpleYesNoDialog::new(
                        &t!("tl_update_dialog.title"),
                        &dialog_message,
                        move |ok| {
                            if !ok {
                                // Remember this exact version so it doesn't re-prompt
                                // until a newer one ships or the user checks manually.
                                updater.skip_update(etag_to_skip);
                                updater.clear_pending_update();
                                return;
                            }
                            updater.run();
                        },
                    )));
            } else {
                self.clear_pending_update();
            }
        } else {
            if let Some(etag) = new_etag {
                let mut updated_cache = repo_cache;
                updated_cache.index_etag = Some(etag);
                let _ = utils::write_json_file(&updated_cache, &cache_path);
            }

            // No main TL updates — check for mod/addon updates unless this was a dedicated main pedantic run.
            let config = hachimi.config.load();
            let mut mod_updates_found = false;
            if !config.disable_mod_downloads && !pedantic_main {
                if let Some(mod_index_url) = &config.translation_repo_index_mod {
                    let ld_dir_path = hachimi.get_active_tl_dir().or_else(|| {
                        config.localized_data_dir.as_ref().map(|p| hachimi.get_data_path(p))
                    });
                    match self.check_for_mod_updates(mod_index_url, pedantic_mod, silent, &config, &ld_dir_path) {
                        Ok(found) => mod_updates_found = found,
                        Err(e) => warn!("Failed to check for mod updates: {}", e),
                    }
                }
            }

            if !mod_updates_found && !silent {
                if let Some(mutex) = Gui::instance() {
                    mutex
                        .lock()
                        .unwrap()
                        .show_notification(&t!("notification.no_tl_updates"));
                }
            }
        }

        Ok(())
    }

    pub fn run(self: Arc<Self>) {
        std::thread::Builder::new()
            .name("tl_repo_updater".into())
            .stack_size(8 * 1024 * 1024) // increase stack size to 8MB to prevent 0xc0000409 (Stack Buffer Overrun) during single-threaded downloads
            .spawn(move || {
                if let Err(e) = self.clone().run_internal() {
                    error!("{}", e);
                    self.progress.store(Arc::new(None));
                    self.is_downloading.store(false, atomic::Ordering::Relaxed);
                    Hachimi::instance().load_localized_data();
                    if let Some(mutex) = Gui::instance() {
                        if let Ok(mut gui) = mutex.lock() {
                            gui.update_progress_visible = false;
                            gui.show_notification(&t!(
                                "notification.update_failed",
                                reason = e.to_string()
                            ));
                        }
                    }
                }
            })
            .expect("Failed to spawn updater thread");
    }

    fn create_dir(path: &Path, override_exists: bool) -> Result<(), Error> {
        if override_exists {
            // rm -rf
            if let Ok(meta) = fs::metadata(path) {
                if meta.is_dir() {
                    fs::remove_dir_all(path)?;
                }
            }
        }

        // mkdir -p
        fs::create_dir_all(path)?;
        Ok(())
    }

    fn run_internal(self: Arc<Self>) -> Result<(), Error> {
        let Ok(_run_guard) = self.run_mutex.try_lock() else {
            info!("Update already in progress, skipping.");
            return Ok(());
        };
        let Some(update_info) = (**self.new_update.load()).clone() else {
            return Ok(());
        };
        self.new_update.store(Arc::new(None));
        self.last_progress_ms.store(0, atomic::Ordering::Relaxed);

        self.progress
            .store(Arc::new(Some(UpdateProgress::new(0, update_info.size))));
        self.is_downloading.store(true, atomic::Ordering::Relaxed);
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = true;
            }
        }

        // Empty the localized data so files couldnt be accessed while update is in progress
        let hachimi = Hachimi::instance();
        hachimi
            .localized_data
            .store(Arc::new(LocalizedData::default()));

        let config = hachimi.config.load();
        let localized_data_dir = hachimi
            .get_active_tl_dir()
            .expect("Active TL repo directory not set.");

        let disk_check_path = localized_data_dir.parent().unwrap_or(Path::new("."));
        check_available_disk_space(disk_check_path, update_info.size as u64)?;

        if update_info.is_new_repo {
            Self::create_dir(&localized_data_dir, true)?;
        } else {
            Self::create_dir(&localized_data_dir, false)?;
        }

        // Download the files - use the pre-determined strategy
        let cached_files = Arc::new(Mutex::new(update_info.cached_files.clone()));
        let error_count = if update_info.will_use_zip {
            self.clone()
                .download_zip(&update_info, &localized_data_dir, cached_files.clone())
        } else {
            self.clone().download_incremental(
                &update_info,
                &localized_data_dir,
                cached_files.clone(),
            )
        }?;
        if error_count > 0 {
            return Err(Error::RuntimeError(format!(
                "{} errors occurred during update",
                error_count
            )));
        }

        if config.apply_atlas_workaround && (update_info.modifies_atlas || update_info.will_use_zip)
        {
            let mut new_config = (**config).clone();
            new_config.apply_atlas_workaround = false;
            hachimi.save_and_reload_config(new_config)?;
            if let Some(gui_mutex) = Gui::instance() {
                gui_mutex
                    .lock()
                    .unwrap()
                    .show_notification(&t!("notification.atlas_workaround_reset"));
            }
        }

        // Drop the download state and mark downloading complete
        self.progress.store(Arc::new(None));
        self.is_downloading.store(false, atomic::Ordering::Relaxed);

        // Reload the localized data
        hachimi.load_localized_data();

        // Save the repo cache (done last so if any of the previous fails, the entire update would be voided)
        let repo_cache = RepoCache {
            base_url: update_info.base_url.clone(),
            index_etag: update_info.index_etag.clone(),
            files: cached_files.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        };
        let cache_path = hachimi
            .config
            .load()
            .selected_tl_repo_id
            .map(get_repo_cache_path)
            .unwrap_or_else(|| hachimi.get_data_path(REPO_CACHE_FILENAME));
        utils::write_json_file(&repo_cache, &cache_path)?;

        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.show_notification(&t!("notification.update_completed"));
                if error_count > 0 {
                    gui.show_notification(&t!(
                        "notification.errors_during_update",
                        count = error_count
                    ));
                }
            }
        }

        // After main TL update completes, check for addon updates (non-pedantic, silent).
        // Pedantic TL checks are scoped to TL only and don't cascade.
        let config = hachimi.config.load();
        if !update_info.pedantic && !config.disable_mod_downloads {
            if let Some(mod_index_url) = &config.translation_repo_index_mod {
                let ld_dir_path = hachimi.get_active_tl_dir().or_else(|| {
                    config.localized_data_dir.as_ref().map(|p| hachimi.get_data_path(p))
                });
                if let Err(e) = self.check_for_mod_updates(mod_index_url, false, true, &config, &ld_dir_path) {
                    warn!("Failed to check for addon updates after TL download: {}", e);
                }
            }
        }

        Ok(())
    }

    fn check_for_mod_updates(
        &self,
        mod_index_url: &str,
        pedantic: bool,
        silent: bool,
        config: &crate::core::hachimi::Config,
        ld_dir_path: &Option<PathBuf>,
    ) -> Result<bool, Error> {
        let hachimi = Hachimi::instance();
        let ld_dir_path = ld_dir_path
            .clone()
            .or_else(|| Some(hachimi.get_data_path(LOCALIZED_DATA_DIR)));

        let mod_index: RepoIndex = match http::get_json(mod_index_url) {
            Ok(idx) => idx,
            Err(e) => {
                warn!("Failed to fetch mod index: {}", e);
                return Ok(false);
            }
        };

        let mod_cache_path = hachimi.get_data_path(REPO_CACHE_MOD_FILENAME);
        let mut mod_cache: RepoCache = if fs::metadata(&mod_cache_path).is_ok() {
            let json = fs::read_to_string(&mod_cache_path)?;
            serde_json::from_str(&json)?
        } else {
            RepoCache::default()
        };

        if mod_cache.files.is_empty() {
            if let Some(ref ld_dir_path) = ld_dir_path {
                let mut inferred_files = FnvHashMap::default();
                debug!("Mod cache is empty, attempting to infer {} files from disk", mod_index.files.len());
                for file in mod_index.files.iter() {
                    let full_path = ld_dir_path.join(&file.path);
                    let is_file = full_path.is_file();
                    let size_match = fs::metadata(&full_path)
                        .map(|m| m.len() as usize == file.size)
                        .unwrap_or(false);
                    let hash_match = if is_file && size_match { 
                        file.verify_integrity(&full_path) 
                    } else { 
                        false 
                    };
                    
                    if is_file && size_match && hash_match {
                        inferred_files.insert(file.path.clone(), file.hash.clone());
                    } else if is_file {
                        debug!("Mod file candidate rejected: {} (is_file={}, size_match={}, hash_match={})", 
                            file.path, is_file, size_match, hash_match);
                    }
                }
                if !inferred_files.is_empty() {
                    info!("Inferred {} mod files from disk, persisting cache", inferred_files.len());
                    mod_cache.base_url = mod_index.base_url.clone();
                    mod_cache.files = inferred_files;
                    // Persist the inferred cache so we don't need to re-verify on next startup
                    let _ = utils::write_json_file(&mod_cache, &mod_cache_path);
                } else {
                    debug!("No mod files could be inferred from disk, {} files checked from index", mod_index.files.len());
                }
            } else {
                warn!("Mod cache is empty and localized_data_dir is not configured");
            }
        }

        // An empty addon cache is not a new repo; it simply means the cache has to be rebuilt
        // from the currently installed addon files on disk. That lets pedantic checks verify
        // integrity instead of forcing a full repo download loop.
        let is_new_mod = !mod_cache.files.is_empty() && mod_index.base_url != mod_cache.base_url;
        let mut mod_cache_files = mod_cache.files.clone();
        debug!("Mod cache state: is_new_mod={}, cached_files_count={}, index_files_count={}", 
            is_new_mod, mod_cache_files.len(), mod_index.files.len());
        if let Some(ref ld_dir) = ld_dir_path {
            Self::populate_existing_mod_files(&mut mod_cache_files, &mod_index.files, ld_dir);
        }

        let mut update_files: Vec<RepoFile> = Vec::new();
        let mut update_size: usize = 0;
        let mut total_size: usize = 0;
        let mut update_reasons: Vec<String> = Vec::new();

        let total_files = mod_index.files.len().max(1);
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = true;
            }
        }

        let result: Result<(), crate::core::Error> = (|| {
            for (i, file) in mod_index.files.iter().enumerate() {
                if i % 50 == 0 {
                    self.mod_progress
                        .store(Arc::new(Some(UpdateProgress::new(i, total_files))));
                }
                if file.path.contains("..") || Path::new(&file.path).has_root() {
                    warn!("Mod file path '{}' sanitized", file.path);
                    continue;
                }

                let mut reason = None;
                let updated = if is_new_mod {
                    reason = Some("new repo".to_string());
                    true
                } else if !pedantic && config.lazy_translation_updates {
                    if let Some(hash) = mod_cache_files.get(&file.path) {
                        if hash != &file.hash {
                            reason = Some("cached hash differs from repo".to_string());
                            true
                        } else {
                            false
                        }
                    } else {
                        reason = Some("not in mod cache".to_string());
                        true
                    }
                } else {
                    let path = ld_dir_path.as_ref().map(|p| p.join(&file.path));
                    let exists = path.as_ref().map(|p| p.is_file()).unwrap_or(false);

                    if let Some(hash) = mod_cache_files.get(&file.path) {
                        if let Some(path) = path {
                            if !exists {
                                reason = Some("file missing on disk".to_string());
                                true
                            } else if hash != &file.hash {
                                reason = Some("cached hash differs from repo".to_string());
                                true
                            } else if fs::metadata(&path)
                                .map(|m| m.len() as usize != file.size)
                                .unwrap_or(true)
                            {
                                reason = Some("file size mismatch".to_string());
                                true
                            } else if pedantic {
                                reason = Some("pedantic integrity mismatch".to_string());
                                !file.verify_integrity(&path)
                            } else {
                                false
                            }
                        } else {
                            reason = Some("invalid disk path".to_string());
                            true
                        }
                    } else {
                        reason = Some("not in mod cache".to_string());
                        true
                    }
                };

                if updated {
                    if let Some(reason) = reason {
                        update_reasons.push(format!("{} => {}", file.path, reason));
                    } else {
                        update_reasons.push(format!("{} => reason unknown", file.path));
                    }
                    update_files.push(file.clone());
                    update_size += file.size;
                }
                total_size += file.size;
            }
            Ok(())
        })();

        // Ensure progress flag cleanup regardless of success or error
        self.mod_progress.store(Arc::new(None));
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = false;
            }
        }

        result?;


        if !update_files.is_empty() {
            if !update_reasons.is_empty() {
                if update_files.len() <= 25 {
                    info!(
                        "Mod update reasons ({} files): {}",
                        update_reasons.len(),
                        update_reasons.join(", ")
                    );
                } else {
                    debug!(
                        "Mod update reasons ({} files): {}",
                        update_reasons.len(),
                        update_reasons.join(", ")
                    );
                }
            }
            info!("Mod updates detected: {} files need updating out of {} total", update_files.len(), mod_index.files.len());
            info!("  Detected as new_mod: {}, lazy_updates enabled: {}", is_new_mod, !pedantic && config.lazy_translation_updates);
            let will_use_zip = Self::should_use_zip_download(
                update_files.len(),
                update_size,
                total_size,
                &mod_index.base_url,
            );
            let actual_download_size = if will_use_zip { total_size } else { update_size };

            self.new_mod_update.store(Arc::new(Some(ModUpdateInfo {
                base_url: mod_index.base_url,
                zip_url: mod_index.zip_url,
                zip_dir: mod_index.zip_dir,
                files: update_files,
                cached_files: mod_cache_files,
                size: actual_download_size,
                update_size,
                total_size,
                will_use_zip,
            })));

            if silent || Gui::instance().is_none() {
                Hachimi::instance().tl_updater.clone().run_mod();
            } else if let Some(mutex) = Gui::instance() {
                let dialog_message = t!(
                    "tl_update_dialog.content_mod",
                    size = Size::from_bytes(actual_download_size)
                );
                mutex
                    .lock()
                    .unwrap()
                    .show_window(Box::new(SimpleYesNoDialog::new(
                        &t!("tl_update_dialog.title_mod"),
                        &dialog_message,
                        |ok| {
                            if !ok {
                                Hachimi::instance().tl_updater.clear_pending_mod_update();
                                return;
                            }
                            Hachimi::instance().tl_updater.clone().run_mod();
                        },
                    )));
            }
            return Ok(true);
        }

        Ok(false)
    }

    pub fn run_mod(self: Arc<Self>) {
        std::thread::Builder::new()
            .name("tl_repo_mod_updater".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                if let Err(e) = self.clone().run_mod_internal() {
                    error!("{}", e);
                    self.mod_progress.store(Arc::new(None));
                    self.is_downloading.store(false, atomic::Ordering::Relaxed);
                    Hachimi::instance().load_localized_data();
                    if let Some(mutex) = Gui::instance() {
                        if let Ok(mut gui) = mutex.lock() {
                            gui.update_progress_visible = false;
                            gui.show_notification(&t!(
                                "notification.update_failed",
                                reason = e.to_string()
                            ));
                        }
                    }
                }
            })
            .expect("Failed to spawn mod updater thread");
    }

    fn run_mod_internal(self: Arc<Self>) -> Result<(), Error> {
        let Ok(_run_guard) = self.run_mutex.try_lock() else {
            info!("Update already in progress, skipping.");
            return Ok(());
        };
        let Some(mod_info) = (**self.new_mod_update.load()).clone() else {
            return Ok(());
        };
        self.new_mod_update.store(Arc::new(None));
        self.last_mod_progress_ms.store(0, atomic::Ordering::Relaxed);

        // Reuse UpdateInfo/download machinery via a temporary UpdateInfo
        let update_info = UpdateInfo {
            base_url: mod_info.base_url.clone(),
            zip_url: mod_info.zip_url.clone(),
            zip_dir: mod_info.zip_dir.clone(),
            files: mod_info.files.clone(),
            is_new_repo: false,
            pedantic: false,
            cached_files: mod_info.cached_files.clone(),
            size: mod_info.size,
            update_size: mod_info.update_size,
            total_size: mod_info.total_size,
            will_use_zip: mod_info.will_use_zip,
            modifies_atlas: false,
            index_etag: None,
        };

        self.mod_progress
            .store(Arc::new(Some(UpdateProgress::new(0, update_info.size))));
        self.is_downloading.store(true, atomic::Ordering::Relaxed);
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = true;
            }
        }

        let hachimi = Hachimi::instance();
        hachimi.localized_data.store(Arc::new(LocalizedData::default()));

        let config = hachimi.config.load();
        let localized_data_dir = hachimi.get_active_tl_dir().unwrap_or_else(|| {
            config
                .localized_data_dir
                .as_ref()
                .map(|p| hachimi.get_data_path(p))
                .unwrap_or_else(|| hachimi.get_data_path(LOCALIZED_DATA_DIR))
        });

        let disk_check_path = localized_data_dir.parent().unwrap_or(Path::new("."));
        check_available_disk_space(disk_check_path, update_info.size as u64)?;

        Self::create_dir(&localized_data_dir, false)?;

        let cached_files = Arc::new(Mutex::new(update_info.cached_files.clone()));

        // Temporarily redirect progress updates to mod_progress via a wrapper updater reference.
        // We drive downloads through the shared download helpers but track progress on mod_progress.
        let error_count = if update_info.will_use_zip {
            self.clone().download_zip_for_mod(&update_info, &localized_data_dir, cached_files.clone())
        } else {
            self.clone().download_incremental_for_mod(&update_info, &localized_data_dir, cached_files.clone())
        }?;

        self.mod_progress.store(Arc::new(None));
        self.is_downloading.store(false, atomic::Ordering::Relaxed);
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.update_progress_visible = false;
            }
        }

        if error_count > 0 {
            warn!("Mod update completed with {} errors (non-fatal), cache will be saved with successfully verified files", error_count);
        }

        if config.localized_data_dir.is_none() {
            let mut new_config = (**config).clone();
            new_config.localized_data_dir = Some(LOCALIZED_DATA_DIR.to_owned());
            hachimi.save_and_reload_config(new_config)?;
        }

        hachimi.load_localized_data();

        let mod_cache = RepoCache {
            base_url: mod_info.base_url.clone(),
            index_etag: None,
            files: cached_files.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        };
        
        let cached_count = mod_cache.files.len();
        info!("Saving mod cache with {} files", cached_count);
        
        let mod_cache_path = hachimi.get_data_path(REPO_CACHE_MOD_FILENAME);
        utils::write_json_file(&mod_cache, &mod_cache_path)?;
        
        info!("Mod cache saved successfully");

        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.show_notification(&t!("notification.mod_update_completed"));
            }
        }

        Ok(())
    }

    fn download_incremental_for_mod(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        let total_size = update_info.size;
        let current_bytes = Arc::new(AtomicUsize::new(0));
        let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
        let fatal_error = Arc::new(Mutex::new(None::<Error>));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let shared_agent: ureq::Agent = ureq::Agent::new_with_config(ureq_config());
        let (sender, receiver) = mpsc::channel::<RepoFile>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut handles = Vec::with_capacity(*NUM_THREADS);
        for _ in 0..*NUM_THREADS {
            let updater = self.clone();
            let localized_data_dir_clone = localized_data_dir.to_path_buf();
            let base_url_clone = update_info.base_url.clone();
            let cached_files_clone = Arc::clone(&cached_files);
            let current_bytes_clone = Arc::clone(&current_bytes);
            let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
            let fatal_error_clone = Arc::clone(&fatal_error);
            let stop_signal_clone = Arc::clone(&stop_signal);
            let receiver_clone = Arc::clone(&receiver);
            let thread_agent = shared_agent.clone();

            let handle = thread::Builder::new()
                .name("mod_incremental_downloader".into())
                .stack_size(8 * 1024 * 1024)
                .spawn(move || {
                    if let Err(e) = thread_priority::set_current_thread_priority(ThreadPriority::Min) {
                        warn!("Failed to set background thread priority for mod incremental downloader: {:?}", e);
                    }
                    let mut job = DownloadJob::new(thread_agent);

                    while let Ok(repo_file) = receiver_clone.lock().unwrap_or_else(|e| e.into_inner()).recv() {
                        if stop_signal_clone.load(atomic::Ordering::Relaxed) { break; }

                        let file_path = repo_file.get_fs_path(&localized_data_dir_clone);
                        let url = utils::concat_unix_path(&base_url_clone, &repo_file.path);
                        job.hasher.reset();

                        let execute_result = (|| -> Result<String, Error> {
                            if let Some(parent) = Path::new(&file_path).parent() {
                                Self::create_dir(parent, false)?;
                            }

                            let mut last_err = None;
                            for attempt in 0..3 {
                                if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                                    break;
                                }

                                job.hasher.reset();
                                let mut file = fs::File::create(&file_path)?;
                                let mut req = job.agent.get(&url);
                                if attempt > 0 {
                                    req = req.header("Cache-Control", "no-cache").header("Pragma", "no-cache");
                                }

                                let res = match req.call() {
                                    Ok(r) => r,
                                    Err(e) => {
                                        last_err = Some(Error::from(e));
                                        let _ = fs::remove_file(&file_path);
                                        thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                                        continue;
                                    }
                                };

                                let mut downloaded_bytes_this_attempt = 0usize;
                                let download_res = http::download_file_buffered(
                                    res,
                                    &mut file,
                                    &mut job.buffer,
                                    |bytes| {
                                        job.hasher.update(bytes);
                                        downloaded_bytes_this_attempt += bytes.len();
                                        let prev_size = current_bytes_clone
                                            .fetch_add(bytes.len(), atomic::Ordering::Relaxed);
                                        store_progress(
                                            &updater.mod_progress,
                                            &updater.last_mod_progress_ms,
                                            prev_size + bytes.len(),
                                            total_size,
                                        );
                                    },
                                );

                                if let Err(e) = download_res {
                                    current_bytes_clone.fetch_sub(downloaded_bytes_this_attempt, atomic::Ordering::Relaxed);
                                    last_err = Some(e);
                                    let _ = fs::remove_file(&file_path);
                                    thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                                    continue;
                                }

                                let _ = file.flush();
                                let _ = file.sync_data();
                                drop(file);

                                let hash = job.hasher.finalize().to_hex().to_string();
                                if hash == repo_file.hash {
                                    job.hasher.reset();
                                    return Ok(hash);
                                }

                                current_bytes_clone.fetch_sub(downloaded_bytes_this_attempt, atomic::Ordering::Relaxed);
                                let path_str = file_path.to_string_lossy().to_string();
                                warn!(
                                    "Mod download hash mismatch on attempt {} for '{}': expected {} got {}",
                                    attempt + 1,
                                    file_path.display(),
                                    repo_file.hash,
                                    hash
                                );
                                let _ = fs::remove_file(&file_path);
                                last_err = Some(Error::FileHashMismatch(path_str));
                                thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                            }

                            if let Some(err) = last_err {
                                if let Error::FileHashMismatch(ref path_str) = err {
                                    Self::log_corrupted_download(&file_path, &url, &repo_file.hash, "mismatch_after_retries");
                                    let _ = fs::remove_file(&file_path);
                                    return Err(Error::FileHashMismatch(path_str.clone()));
                                }
                                Err(err)
                            } else {
                                Err(Error::FileHashMismatch(file_path.to_string_lossy().to_string()))
                            }
                        })();

                        match execute_result {
                            Ok(hash) => {
                                cached_files_clone.lock().unwrap_or_else(|e| e.into_inner()).insert(repo_file.path.clone(), hash);
                            }
                            Err(e) => {
                                if matches!(e, Error::OutOfDiskSpace) {
                                    error!("Fatal error during mod incremental download of '{}': {}", file_path.display(), e);
                                    *fatal_error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                                    stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                    return;
                                } else {
                                    error!("Non-fatal error during mod incremental download of '{}': {}", file_path.display(), e);
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                }
                            }
                        }
                    }
                })
                .unwrap();
            handles.push(handle);
        }

        for repo_file in update_info.files.iter() {
            if sender.send(repo_file.clone()).is_err() { break; }
        }
        drop(sender);
        for handle in handles {
            let _ = handle.join();
        }

        if let Some(err) = fatal_error.lock().unwrap_or_else(|e| e.into_inner()).take() {
            return Err(err);
        }
        Ok(non_fatal_error_count.load(atomic::Ordering::Relaxed))
    }

    fn download_zip_for_mod(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        info!("Starting mod ZIP download for {} files, will use ZIP archive", update_info.files.len());
        let zip_path = localized_data_dir.join(".tmp_mod.zip");
        let _zip_cleanup = ZipCleanupGuard(&zip_path);
        #[allow(unused_assignments)]
        let mut error_count = 0;

        {
            let total_size_header = ureq::Agent::new_with_config(ureq_config())
                .head(&update_info.zip_url)
                .call()
                .ok()
                .and_then(|res| {
                    res.headers()
                        .get("Content-Length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<usize>().ok())
                });

            let progress_total = match total_size_header {
                Some(size) if size > 0 => size,
                _ => update_info.size,
            };

            let downloaded = Arc::new(AtomicUsize::new(0));
            let self_clone = self.clone();
            let downloaded_clone = downloaded.clone();

            let progress_bar = Arc::new(move |bytes_read: usize| {
                let prev_size = downloaded_clone.fetch_add(bytes_read, atomic::Ordering::Relaxed);
                let current = prev_size + bytes_read;
                store_progress(&self_clone.mod_progress, &self_clone.last_mod_progress_ms, current, progress_total);
            });

            http::download_file_parallel(
                &update_info.zip_url,
                &zip_path,
                *NUM_THREADS,
                MIN_CHUNK_SIZE,
                CHUNK_SIZE,
                progress_bar,
            )?;

            let files_to_extract = Arc::new(
                update_info
                    .files
                    .iter()
                    .map(|f| (utils::concat_unix_path(&update_info.zip_dir, &f.path), f.clone()))
                    .collect::<FnvHashMap<_, _>>(),
            );
            info!("Prepared {} mod files for extraction, zip_dir prefix: {}", files_to_extract.len(), update_info.zip_dir);
            let zip_url_clone = update_info.zip_url.clone();

            let zip_file = fs::File::open(&zip_path)?;
            let mmap = Arc::new(unsafe { memmap2::Mmap::map(&zip_file)? });

            let total_size = update_info.size;
            let current_bytes = Arc::new(AtomicUsize::new(0));
            let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
            let fatal_error = Arc::new(Mutex::new(None::<Error>));
            let stop_signal = Arc::new(AtomicBool::new(false));

            let (sender, receiver) = mpsc::channel::<usize>();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut handles = Vec::with_capacity(*NUM_THREADS);
            let extraction_in_progress: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

            for _ in 0..*NUM_THREADS {
                let updater = self.clone();
                let mmap_thread = Arc::clone(&mmap);
                let files_to_extract_clone = Arc::clone(&files_to_extract);
                let localized_data_dir_clone = localized_data_dir.to_path_buf();
                let zip_dir_clone = update_info.zip_dir.clone();
                let cached_files_clone = Arc::clone(&cached_files);
                let current_bytes_clone = Arc::clone(&current_bytes);
                let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
                let fatal_error_clone = Arc::clone(&fatal_error);
                let stop_signal_clone = Arc::clone(&stop_signal);
                let receiver_clone = Arc::clone(&receiver);
                let zip_url_clone = zip_url_clone.clone();

                let extraction_in_progress = Arc::clone(&extraction_in_progress);

                let handle = thread::Builder::new()
                    .name("mod_zip_extractor".into())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        if let Err(e) = thread_priority::set_current_thread_priority(ThreadPriority::Min) {
                            warn!("Failed to set background thread priority for mod zip extractor: {:?}", e);
                        }
                        let mut archive = match zip::ZipArchive::new(Cursor::new(&mmap_thread[..])) {
                            Ok(a) => a,
                            Err(e) => {
                                error!("Failed to create ZipArchive in mod extraction thread: {}", e);
                                return;
                            }
                        };
                        let mut buffer = vec![0u8; CHUNK_SIZE];
                        let mut hasher = blake3::Hasher::new();

                        while let Ok(i) = receiver_clone.lock().unwrap_or_else(|e| e.into_inner()).recv() {
                            if stop_signal_clone.load(atomic::Ordering::Relaxed) { break; }

                            let mut zip_entry = match archive.by_index(i) {
                                Ok(entry) => entry,
                                Err(_) => { non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed); continue; }
                            };

                            let repo_file = match Self::resolve_zip_entry_repo_file(
                                &files_to_extract_clone,
                                zip_entry.name(),
                                &zip_dir_clone,
                            ) {
                                Some(file) => file.clone(),
                                None => {
                                    debug!("ZIP entry not in mod files_to_extract list: {} (searching {} entries)", zip_entry.name(), files_to_extract_clone.len());
                                    continue;
                                }
                            };

                            let path = repo_file.get_fs_path(&localized_data_dir_clone);
                            debug!("ZIP entry '{}' -> target '{}'", zip_entry.name(), path.display());

                            // Prevent multiple threads from extracting the same target path concurrently
                            {
                                let mut inprog = extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner());
                                if !inprog.insert(repo_file.path.clone()) {
                                    debug!("Skipping duplicate extraction for {}", repo_file.path);
                                    continue;
                                }
                            }
                            if let Some(parent) = path.parent() {
                                if Self::create_dir(parent, false).is_err() {
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                    continue;
                                }
                            }

                            // Write to a temporary file and atomically rename into place to avoid partial/overwritten files
                            let tmp_path = path.with_extension("tmp_mod");
                            let mut out_file = match fs::File::create(&tmp_path) {
                                Ok(f) => f,
                                Err(_) => {
                                    extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                    continue;
                                }
                            };

                            loop {
                                match zip_entry.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let data = &buffer[..n];
                                        if out_file.write_all(data).is_err() {
                                                    extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                                    *fatal_error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(Error::OutOfDiskSpace);
                                                    stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                                    return;
                                        }
                                        hasher.update(data);
                                        let prev = current_bytes_clone.fetch_add(n, atomic::Ordering::Relaxed);
                                        store_progress(&updater.mod_progress, &updater.last_mod_progress_ms, prev + n, total_size);
                                    }
                                    Err(_) => {
                                        let _ = fs::remove_file(&tmp_path);
                                        non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                        break;
                                    }
                                }
                            }

                            let _ = out_file.flush();
                            let _ = out_file.sync_data();
                            drop(out_file);

                            let hash = hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                Self::log_corrupted_download(&path, &zip_url_clone, &repo_file.hash, &hash);
                                let _ = fs::remove_file(&tmp_path);
                                warn!("Hash mismatch for mod file '{}': expected {} but got {}", repo_file.path, repo_file.hash, hash);
                                non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                            } else {
                                // Atomically replace target with tmp file
                                if let Err(e) = fs::rename(&tmp_path, &path) {
                                    error!("Failed to rename '{}' -> '{}': {}", tmp_path.display(), path.display(), e);
                                    let _ = fs::remove_file(&tmp_path);
                                    non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                } else {
                                    cached_files_clone.lock().unwrap_or_else(|e| e.into_inner()).insert(repo_file.path.clone(), hash.clone());
                                    info!("Extracted mod file '{}' -> {} (hash={})", repo_file.path, path.display(), hash);
                                }
                            }
                            hasher.reset();

                            // Clear in-progress marker
                            extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                        }
                    })
                    .unwrap();
                handles.push(handle);
            }

            let zip_len = zip::ZipArchive::new(Cursor::new(&mmap[..]))?.len();
            debug!("Mod ZIP archive has {} entries, spawned {} extraction threads", zip_len, *NUM_THREADS);
            for i in 0..zip_len {
                if sender.send(i).is_err() { 
                    warn!("Failed to send ZIP entry index {}, receiver may have closed", i);
                    break; 
                }
            }
            drop(sender);
            for handle in handles {
                let _ = handle.join();
            }

            if let Some(err) = fatal_error.lock().unwrap_or_else(|e| e.into_inner()).take() {
                let _ = fs::remove_file(&zip_path);
                return Err(err);
            }
            error_count = non_fatal_error_count.load(atomic::Ordering::Relaxed);
        }

        if let Err(e) = fs::remove_file(&zip_path) {
            error!("Failed to remove temporary mod zip '{}': {}", zip_path.display(), e);
        }

        let final_cached_count = cached_files.lock().unwrap_or_else(|e| e.into_inner()).len();
        info!("Mod ZIP extraction complete: {} files cached, {} errors", final_cached_count, error_count);

        Ok(error_count)
    }

    fn download_incremental(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        let total_size = update_info.size;
        let current_bytes = Arc::new(AtomicUsize::new(0));
        let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
        let fatal_error = Arc::new(Mutex::new(None::<Error>));
        let stop_signal = Arc::new(AtomicBool::new(false));

        let shared_agent: ureq::Agent = ureq::Agent::new_with_config(ureq_config());

        let (sender, receiver) = mpsc::channel::<RepoFile>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut handles = Vec::with_capacity(*NUM_THREADS);
        for _ in 0..*NUM_THREADS {
            let updater = self.clone();
            let localized_data_dir_clone = localized_data_dir.to_path_buf();
            let base_url_clone = update_info.base_url.clone();
            let cached_files_clone = Arc::clone(&cached_files);
            let current_bytes_clone = Arc::clone(&current_bytes);
            let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
            let fatal_error_clone = Arc::clone(&fatal_error);
            let stop_signal_clone = Arc::clone(&stop_signal);
            let receiver_clone = Arc::clone(&receiver);

            let thread_agent = shared_agent.clone();

            let handle = thread::Builder::new()
                .name("incremental_downloader".into())
                .stack_size(8 * 1024 * 1024)
                .spawn(move || {
                    if let Err(e) = thread_priority::set_current_thread_priority(ThreadPriority::Min) {
                        warn!("Failed to set background thread priority for incremental downloader: {:?}", e);
                    }
                    let mut job = DownloadJob::new(thread_agent);

                    while let Ok(repo_file) = receiver_clone.lock().unwrap_or_else(|e| e.into_inner()).recv() {
                        if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                            break;
                        }

                        let file_path = repo_file.get_fs_path(&localized_data_dir_clone);
                        let url = utils::concat_unix_path(&base_url_clone, &repo_file.path);

                        let execute_result = (|| -> Result<String, Error> {
                            if let Some(parent) = Path::new(&file_path).parent() {
                                Self::create_dir(parent, false)?;
                            }

                            let mut last_err = None;
                            for attempt in 0..3 {
                                if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                                    break;
                                }

                                job.hasher.reset();
                                let mut file = fs::File::create(&file_path)?;
                                let mut req = job.agent.get(&url);
                                if attempt > 0 {
                                    req = req.header("Cache-Control", "no-cache").header("Pragma", "no-cache");
                                }

                                let res = match req.call() {
                                    Ok(r) => r,
                                    Err(e) => {
                                        last_err = Some(Error::from(e));
                                        Self::cleanup_partial_file(&file_path);
                                        thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                                        continue;
                                    }
                                };

                                let mut downloaded_bytes_this_attempt = 0usize;
                                let download_res = http::download_file_buffered(
                                    res,
                                    &mut file,
                                    &mut job.buffer,
                                    |bytes| {
                                        job.hasher.update(bytes);
                                        downloaded_bytes_this_attempt += bytes.len();
                                        let prev_size = current_bytes_clone
                                            .fetch_add(bytes.len(), atomic::Ordering::Relaxed);
                                        store_progress(
                                            &updater.progress,
                                            &updater.last_progress_ms,
                                            prev_size + bytes.len(),
                                            total_size,
                                        );
                                    },
                                );

                                if let Err(e) = download_res {
                                    current_bytes_clone.fetch_sub(downloaded_bytes_this_attempt, atomic::Ordering::Relaxed);
                                    last_err = Some(e);
                                    Self::cleanup_partial_file(&file_path);
                                    thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                                    continue;
                                }

                                let _ = file.flush();
                                let _ = file.sync_data();
                                drop(file);

                                let hash = job.hasher.finalize().to_hex().to_string();
                                if hash == repo_file.hash {
                                    job.hasher.reset();
                                    return Ok(hash);
                                }

                                current_bytes_clone.fetch_sub(downloaded_bytes_this_attempt, atomic::Ordering::Relaxed);
                                let path_str = file_path.to_string_lossy().to_string();
                                warn!(
                                    "Download hash mismatch on attempt {} for '{}': expected {} got {}",
                                    attempt + 1,
                                    file_path.display(),
                                    repo_file.hash,
                                    hash
                                );
                                Self::cleanup_partial_file(&file_path);
                                last_err = Some(Error::FileHashMismatch(path_str));
                                thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                            }

                            if let Some(err) = last_err {
                                if let Error::FileHashMismatch(ref path_str) = err {
                                    Self::log_corrupted_download(&file_path, &url, &repo_file.hash, "mismatch_after_retries");
                                    Self::cleanup_partial_file(&file_path);
                                    return Err(Error::FileHashMismatch(path_str.clone()));
                                }
                                Err(err)
                            } else {
                                Err(Error::FileHashMismatch(file_path.to_string_lossy().to_string()))
                            }
                        })();

                        match execute_result {
                            Ok(hash) => {
                                cached_files_clone
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .insert(repo_file.path.clone(), hash);
                            }
                            Err(e) => {
                                if matches!(e, Error::OutOfDiskSpace | Error::FileHashMismatch(_)) {
                                    error!("Fatal error during incremental download of '{}': {}", file_path.display(), e);
                                    *fatal_error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                                    stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                    return;
                                } else {
                                    error!("Non-fatal error during incremental download of '{}': {}", file_path.display(), e);
                                    non_fatal_error_count_clone
                                        .fetch_add(1, atomic::Ordering::Relaxed);
                                }
                            }
                        }
                    }
                })
                .unwrap();
            handles.push(handle);
        }

        for repo_file in update_info.files.iter() {
            if sender.send(repo_file.clone()).is_err() {
                break;
            }
        }
        drop(sender);

        for handle in handles {
            let _ = handle.join();
        }

        if let Some(err) = fatal_error.lock().unwrap_or_else(|e| e.into_inner()).take() {
            return Err(err);
        }

        Ok(non_fatal_error_count.load(atomic::Ordering::Relaxed))
    }

    fn download_zip(
        self: Arc<Self>,
        update_info: &UpdateInfo,
        localized_data_dir: &Path,
        cached_files: Arc<Mutex<FnvHashMap<String, String>>>,
    ) -> Result<usize, Error> {
        let zip_path = localized_data_dir.join(".tmp.zip");
        let _zip_cleanup = ZipCleanupGuard(&zip_path);
        let extraction_in_progress: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        // idk compiler going monkey mode unless i add this
        #[allow(unused_assignments)]
        let mut error_count = 0;

        {
            let total_size_header = ureq::Agent::new_with_config(ureq_config())
                .head(&update_info.zip_url)
                .call()
                .ok()
                .and_then(|res| {
                    res.headers()
                        .get("Content-Length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<usize>().ok())
                });

            let progress_total = match total_size_header {
                Some(size) if size > 0 => {
                    debug!(
                        "Using Content-Length from header for progress bar: {}",
                        size
                    );
                    size
                }
                _ => {
                    debug!("Server did not provide a valid Content-Length. Using fallback size from index: {}", update_info.size);
                    update_info.size
                }
            };

            let downloaded = Arc::new(AtomicUsize::new(0));
            let self_clone = self.clone();
            let downloaded_clone = downloaded.clone();

            let progress_bar = Arc::new(move |bytes_read: usize| {
                let prev_size = downloaded_clone.fetch_add(bytes_read, atomic::Ordering::Relaxed);
                let current = prev_size + bytes_read;
                store_progress(&self_clone.progress, &self_clone.last_progress_ms, current, progress_total);
            });

            http::download_file_parallel(
                &update_info.zip_url,
                &zip_path,
                *NUM_THREADS,
                MIN_CHUNK_SIZE,
                CHUNK_SIZE,
                progress_bar,
            )?;

            let files_to_extract = Arc::new(
                update_info
                    .files
                    .iter()
                    .map(|f| {
                        (
                            utils::concat_unix_path(&update_info.zip_dir, &f.path),
                            f.clone(),
                        )
                    })
                    .collect::<FnvHashMap<_, _>>(),
            );
            let zip_url_clone = update_info.zip_url.clone();

            let zip_file = fs::File::open(&zip_path)?;
            let mmap = Arc::new(unsafe { memmap2::Mmap::map(&zip_file)? });

            let total_size = update_info.size;
            let current_bytes = Arc::new(AtomicUsize::new(0));
            let non_fatal_error_count = Arc::new(AtomicUsize::new(0));
            let fatal_error = Arc::new(Mutex::new(None::<Error>));
            let stop_signal = Arc::new(AtomicBool::new(false));

            let (sender, receiver) = mpsc::channel::<usize>();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut handles = Vec::with_capacity(*NUM_THREADS);

            for _ in 0..*NUM_THREADS {
                let updater = self.clone();
                let mmap_thread = Arc::clone(&mmap);
                let files_to_extract_clone = Arc::clone(&files_to_extract);
                let localized_data_dir_clone = localized_data_dir.to_path_buf();
                let zip_dir_clone = update_info.zip_dir.clone();
                let cached_files_clone = Arc::clone(&cached_files);
                let current_bytes_clone = Arc::clone(&current_bytes);
                let non_fatal_error_count_clone = Arc::clone(&non_fatal_error_count);
                let fatal_error_clone = Arc::clone(&fatal_error);
                let stop_signal_clone = Arc::clone(&stop_signal);
                let receiver_clone = Arc::clone(&receiver);
                let zip_url_clone = zip_url_clone.clone();

                let extraction_in_progress = Arc::clone(&extraction_in_progress);                let handle = thread::Builder::new()
                    .name("zip_extractor".into())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        if let Err(e) = thread_priority::set_current_thread_priority(ThreadPriority::Min) {
                            warn!("Failed to set background thread priority for zip extractor: {:?}", e);
                        }

                        let mut archive = match zip::ZipArchive::new(Cursor::new(&mmap_thread[..]))
                        {
                            Ok(a) => a,
                            Err(_) => return,
                        };

                        let mut buffer = vec![0u8; CHUNK_SIZE];
                        let mut hasher = blake3::Hasher::new();

                        while let Ok(i) = receiver_clone.lock().unwrap_or_else(|e| e.into_inner()).recv() {
                            if stop_signal_clone.load(atomic::Ordering::Relaxed) {
                                break;
                            }

                            let mut zip_entry = match archive.by_index(i) {
                                Ok(entry) => entry,
                                Err(_) => {
                                    non_fatal_error_count_clone
                                        .fetch_add(1, atomic::Ordering::Relaxed);
                                    continue;
                                }
                            };

                            let repo_file = match Self::resolve_zip_entry_repo_file(
                                &files_to_extract_clone,
                                zip_entry.name(),
                                &zip_dir_clone,
                            ) {
                                Some(file) => file.clone(),
                                None => continue,
                            };

                            let path = repo_file.get_fs_path(&localized_data_dir_clone);
                            debug!("ZIP entry '{}' -> target '{}'", zip_entry.name(), path.display());

                            // Prevent multiple threads from extracting the same target path concurrently
                            {
                                let mut inprog = extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner());
                                if !inprog.insert(repo_file.path.clone()) {
                                    debug!("Skipping duplicate extraction for {}", repo_file.path);
                                    continue;
                                }
                            }

                            if let Some(parent) = path.parent() {
                                if Self::create_dir(parent, false).is_err() {
                                    extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                    non_fatal_error_count_clone
                                        .fetch_add(1, atomic::Ordering::Relaxed);
                                    continue;
                                }
                            }

                            let tmp_path = path.with_extension("tmp_mod");
                            let mut out_file = match fs::File::create(&tmp_path) {
                                Ok(file) => file,
                                Err(_) => {
                                    extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                    non_fatal_error_count_clone
                                        .fetch_add(1, atomic::Ordering::Relaxed);
                                    continue;
                                }
                            };

                            loop {
                                match zip_entry.read(&mut buffer) {
                                    Ok(0) => break,
                                    Ok(read_bytes) => {
                                        let data_slice = &buffer[..read_bytes];
                                        if out_file.write_all(data_slice).is_err() {
                                            let _ = fs::remove_file(&tmp_path);
                                            extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                            *fatal_error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(Error::OutOfDiskSpace);
                                            stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                            return;
                                        }
                                        hasher.update(data_slice);
                                        let prev_size = current_bytes_clone.fetch_add(read_bytes, atomic::Ordering::Relaxed);
                                        store_progress(&updater.progress, &updater.last_progress_ms, prev_size + read_bytes, total_size);
                                    }
                                    Err(_) => {
                                        let _ = fs::remove_file(&tmp_path);
                                        non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                                        break;
                                    }
                                }
                            }

                            let _ = out_file.flush();
                            let _ = out_file.sync_data();
                            drop(out_file);

                            let hash = hasher.finalize().to_hex().to_string();
                            if hash != repo_file.hash {
                                Self::log_corrupted_download(&path, &zip_url_clone, &repo_file.hash, &hash);
                                let _ = fs::remove_file(&tmp_path);
                                let path_str = path.to_str().unwrap_or("").to_string();
                                extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                                *fatal_error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(Error::FileHashMismatch(path_str));
                                stop_signal_clone.store(true, atomic::Ordering::Relaxed);
                                return;
                            }

                            if let Err(e) = fs::rename(&tmp_path, &path) {
                                error!("Failed to rename '{}' -> '{}': {}", tmp_path.display(), path.display(), e);
                                let _ = fs::remove_file(&tmp_path);
                                non_fatal_error_count_clone.fetch_add(1, atomic::Ordering::Relaxed);
                            } else {
                                cached_files_clone.lock().unwrap_or_else(|e| e.into_inner()).insert(repo_file.path.clone(), hash.clone());
                                info!("Extracted '{}' -> {} (hash={})", repo_file.path, path.display(), hash);
                            }
                            hasher.reset();

                            extraction_in_progress.lock().unwrap_or_else(|e| e.into_inner()).remove(&repo_file.path);
                        }
                    })
                    .unwrap();
                handles.push(handle);
            }

            let zip_len = zip::ZipArchive::new(Cursor::new(&mmap[..]))?.len();
            for i in 0..zip_len {
                if sender.send(i).is_err() {
                    break;
                }
            }
            drop(sender);

            for handle in handles {
                let _ = handle.join();
            }

            if let Some(err) = fatal_error.lock().unwrap_or_else(|e| e.into_inner()).take() {
                return Err(err);
            }
            error_count = non_fatal_error_count.load(atomic::Ordering::Relaxed);
        }

        if let Err(e) = fs::remove_file(&zip_path) {
            error!(
                "Failed to remove temporary file '{}': {}",
                zip_path.display(),
                e
            );
            error_count += 1;
        }

        Ok(error_count)
    }

    pub fn progress(&self) -> Option<UpdateProgress> {
        (**self.progress.load()).clone()
    }

    pub fn mod_progress(&self) -> Option<UpdateProgress> {
        (**self.mod_progress.load()).clone()
    }

    pub fn is_downloading(&self) -> bool {
        self.is_downloading.load(atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_entry_matches_repo_path_with_localized_data_prefix() {
        let mut files = FnvHashMap::default();
        files.insert(
            "megamtl/localized_data/assets/story/data/04/1047/storytimeline_041047001.json".to_string(),
            RepoFile {
                path: "assets/story/data/04/1047/storytimeline_041047001.json".to_string(),
                hash: "hash".to_string(),
                size: 1,
            },
        );

        let repo_file = super::Updater::resolve_zip_entry_repo_file(
            &files,
            "localized_data/assets/story/data/04/1047/storytimeline_041047001.json",
            "megamtl/localized_data",
        );

        assert!(repo_file.is_some());
        assert_eq!(repo_file.unwrap().path, "assets/story/data/04/1047/storytimeline_041047001.json");
    }
}
