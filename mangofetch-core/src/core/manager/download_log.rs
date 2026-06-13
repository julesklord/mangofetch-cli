use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const MAX_LINES_PER_DOWNLOAD: usize = 200;
const EMIT_THROTTLE_MS: u64 = 200;

struct Entry {
    lines: VecDeque<String>,
    last_emit: Option<Instant>,
    pending_emit: bool,
}

impl Entry {
    fn new() -> Self {
        Self {
            lines: VecDeque::with_capacity(MAX_LINES_PER_DOWNLOAD),
            last_emit: None,
            pending_emit: false,
        }
    }
}

static STORE: OnceLock<Mutex<HashMap<u64, Entry>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<u64, Entry>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn push_line(id: u64, line: &str) -> bool {
    let mut map = match store().lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    let entry = map.entry(id).or_insert_with(Entry::new);
    if entry.lines.len() >= MAX_LINES_PER_DOWNLOAD {
        entry.lines.pop_front();
    }
    entry.lines.push_back(line.to_string());

    let now = Instant::now();
    let should_emit = match entry.last_emit {
        Some(t) => now.duration_since(t) >= Duration::from_millis(EMIT_THROTTLE_MS),
        None => true,
    };
    if should_emit {
        entry.last_emit = Some(now);
        entry.pending_emit = false;
        true
    } else {
        entry.pending_emit = true;
        false
    }
}

pub fn get(id: u64) -> Vec<String> {
    match store().lock() {
        Ok(g) => g
            .get(&id)
            .map(|e| e.lines.iter().cloned().collect())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn clear(id: u64) {
    if let Ok(mut g) = store().lock() {
        g.remove(&id);
    }
}

pub fn clear_all() {
    if let Ok(mut g) = store().lock() {
        g.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_push_and_get() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id = 1001;
        assert!(push_line(id, "line 1")); // First line should be emitted
        assert!(!push_line(id, "line 2")); // Should be throttled
        let lines = get(id);
        assert_eq!(lines, vec!["line 1".to_string(), "line 2".to_string()]);
    }

    #[test]
    fn test_get_nonexistent() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id = 9999;
        let lines = get(id);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_emit_throttling() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id = 1002;
        assert!(push_line(id, "first")); // emits
        assert!(!push_line(id, "second")); // throttled

        // Wait for throttle to expire
        thread::sleep(Duration::from_millis(EMIT_THROTTLE_MS + 10));
        assert!(push_line(id, "third")); // emits again
    }

    #[test]
    fn test_max_lines_limit() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id = 1003;
        for i in 0..(MAX_LINES_PER_DOWNLOAD + 10) {
            push_line(id, &format!("line {}", i));
        }
        let lines = get(id);
        assert_eq!(lines.len(), MAX_LINES_PER_DOWNLOAD);
        // The first 10 lines should have been popped
        assert_eq!(lines.first().unwrap(), "line 10");
        assert_eq!(
            lines.last().unwrap(),
            &format!("line {}", MAX_LINES_PER_DOWNLOAD + 9)
        );
    }

    #[test]
    fn test_clear() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id = 1004;
        push_line(id, "test");
        assert_eq!(get(id).len(), 1);
        clear(id);
        assert_eq!(get(id).len(), 0);
    }

    #[test]
    fn test_clear_all() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let id1 = 1005;
        let id2 = 1006;
        push_line(id1, "test1");
        push_line(id2, "test2");
        assert_eq!(get(id1).len(), 1);
        assert_eq!(get(id2).len(), 1);

        clear_all();

        assert_eq!(get(id1).len(), 0);
        assert_eq!(get(id2).len(), 0);
    }
}
