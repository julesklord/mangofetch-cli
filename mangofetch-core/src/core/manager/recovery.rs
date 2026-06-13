use crate::models::queue::QueueStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const RECOVERY_FILE: &str = "recovery.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryItem {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub platform: String,
    pub output_dir: String,
    pub status: QueueStatus,
    #[serde(default)]
    pub download_mode: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub format_id: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecoveryFile {
    #[serde(default)]
    items: Vec<RecoveryItem>,
}

static STORE: OnceLock<Mutex<HashMap<u64, RecoveryItem>>> = OnceLock::new();
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn get_next_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

fn store() -> &'static Mutex<HashMap<u64, RecoveryItem>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn file_path() -> Option<PathBuf> {
    crate::core::paths::app_data_dir().map(|d| d.join(RECOVERY_FILE))
}

fn write_to_disk(items: &HashMap<u64, RecoveryItem>) {
    let Some(path) = file_path() else { return };
    let Some(parent) = path.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("[recovery] create_dir_all failed: {}", e);
        return;
    }

    let tmp = path.with_extension("json.tmp");
    let write_result = (|| -> anyhow::Result<()> {
        let f = std::fs::File::create(&tmp)?;
        let mut writer = std::io::BufWriter::new(f);

        let file_data = RecoveryFile {
            items: items.values().cloned().collect(),
        };

        serde_json::to_writer_pretty(&mut writer, &file_data)?;
        writer.into_inner()?.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        tracing::warn!("[recovery] write failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
        return;
    }

    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("[recovery] rename failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn init_from_disk() {
    let Some(path) = file_path() else { return };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let parsed: RecoveryFile = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[recovery] parse failed: {}", e);
            return;
        }
    };
    let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
    guard.clear();
    let mut max_id = 0;
    for item in parsed.items {
        if item.id > max_id {
            max_id = item.id;
        }
        guard.insert(item.id, item);
    }
    NEXT_ID.store(max_id + 1, std::sync::atomic::Ordering::SeqCst);
}

pub fn persist(item: RecoveryItem) {
    let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(item.id, item);
    write_to_disk(&guard);
}

pub fn remove(id: u64) {
    let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
    if guard.remove(&id).is_some() {
        write_to_disk(&guard);
    }
}

pub fn list() -> Vec<RecoveryItem> {
    let guard = store().lock().unwrap_or_else(|e| e.into_inner());
    guard.values().cloned().collect()
}

pub fn clear_all() {
    let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
    guard.clear();
    write_to_disk(&guard);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::queue::QueueStatus;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    struct TestEnv {
        dir: std::path::PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let id = uuid::Uuid::new_v4();
            let dir = std::env::temp_dir().join(format!("mangofetch_recovery_test_{}", id));
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_var("MANGOFETCH_DATA_DIR", &dir);
            Self { dir }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
            std::env::remove_var("MANGOFETCH_DATA_DIR");
        }
    }

    fn clear_global_store() {
        let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }

    #[test]
    fn test_persist_and_list() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let _env = TestEnv::new();
        clear_global_store();

        let item = RecoveryItem {
            id: 1,
            url: "http://example.com".to_string(),
            title: "Test Video".to_string(),
            platform: "test".to_string(),
            output_dir: "/tmp".to_string(),
            status: QueueStatus::Queued,
            download_mode: None,
            quality: None,
            format_id: None,
            referer: None,
            file_path: None,
            file_size_bytes: None,
        };

        persist(item.clone());

        let items = list();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, 1);
        assert_eq!(items[0].title, "Test Video");

        let path = file_path().unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Test Video"));
    }

    #[test]
    fn test_remove() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let _env = TestEnv::new();
        clear_global_store();

        let item = RecoveryItem {
            id: 2,
            url: "http://example.com/2".to_string(),
            title: "Test Video 2".to_string(),
            platform: "test".to_string(),
            output_dir: "/tmp".to_string(),
            status: QueueStatus::Queued,
            download_mode: None,
            quality: None,
            format_id: None,
            referer: None,
            file_path: None,
            file_size_bytes: None,
        };

        persist(item);
        assert_eq!(list().len(), 1);

        remove(2);
        assert_eq!(list().len(), 0);

        let path = file_path().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: RecoveryFile = serde_json::from_str(&content).unwrap();
        assert!(parsed.items.is_empty());
    }

    #[test]
    fn test_init_from_disk() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let _env = TestEnv::new();
        clear_global_store();

        let item = RecoveryItem {
            id: 3,
            url: "http://example.com/3".to_string(),
            title: "Disk Test".to_string(),
            platform: "test".to_string(),
            output_dir: "/tmp".to_string(),
            status: QueueStatus::Active,
            download_mode: None,
            quality: None,
            format_id: None,
            referer: None,
            file_path: None,
            file_size_bytes: None,
        };

        persist(item);

        clear_global_store();
        assert_eq!(list().len(), 0);

        init_from_disk();
        let items = list();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, 3);
        assert_eq!(items[0].title, "Disk Test");

        assert_eq!(get_next_id(), 4);
    }

    #[test]
    fn test_clear_all() {
        let _lock = TEST_MUTEX.lock().unwrap();
        let _env = TestEnv::new();
        clear_global_store();

        let item1 = RecoveryItem {
            id: 4,
            url: "http://example.com/4".to_string(),
            title: "Test 4".to_string(),
            platform: "test".to_string(),
            output_dir: "/tmp".to_string(),
            status: QueueStatus::Queued,
            download_mode: None,
            quality: None,
            format_id: None,
            referer: None,
            file_path: None,
            file_size_bytes: None,
        };

        let item2 = RecoveryItem {
            id: 5,
            url: "http://example.com/5".to_string(),
            title: "Test 5".to_string(),
            platform: "test".to_string(),
            output_dir: "/tmp".to_string(),
            status: QueueStatus::Queued,
            download_mode: None,
            quality: None,
            format_id: None,
            referer: None,
            file_path: None,
            file_size_bytes: None,
        };

        persist(item1);
        persist(item2);
        assert_eq!(list().len(), 2);

        clear_all();
        assert_eq!(list().len(), 0);

        let path = file_path().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: RecoveryFile = serde_json::from_str(&content).unwrap();
        assert!(parsed.items.is_empty());
    }
}
