use std::sync::{Arc, OnceLock};

pub type LogSink = Arc<dyn Fn(u64, &str) + Send + Sync + 'static>;

static SINK: OnceLock<LogSink> = OnceLock::new();

pub fn set_log_sink(sink: LogSink) {
    let _ = SINK.set(sink);
}

pub fn emit_log(id: u64, line: &str) {
    if let Some(s) = SINK.get() {
        s(id, line);
    }
}

tokio::task_local! {
    pub static CURRENT_DOWNLOAD_ID: u64;
}

pub fn current_download_id() -> Option<u64> {
    CURRENT_DOWNLOAD_ID.try_with(|v| *v).ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    static CAPTURED_LOGS: std::sync::OnceLock<Arc<Mutex<Vec<(u64, String)>>>> =
        std::sync::OnceLock::new();
    static INIT_SINK: std::sync::Once = std::sync::Once::new();

    fn setup_test_sink() -> Arc<Mutex<Vec<(u64, String)>>> {
        let logs = CAPTURED_LOGS
            .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
            .clone();

        INIT_SINK.call_once(|| {
            let logs_clone = logs.clone();
            set_log_sink(Arc::new(move |id, line| {
                logs_clone.lock().unwrap().push((id, line.to_string()));
            }));
        });

        logs.lock().unwrap().clear();
        logs
    }

    #[test]
    fn test_set_and_emit_log() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let logs = setup_test_sink();

        emit_log(42, "test log line 1");
        emit_log(42, "test log line 2");
        emit_log(99, "another log");

        let captured = logs.lock().unwrap();
        assert_eq!(captured.len(), 3);
        assert_eq!(captured[0], (42, "test log line 1".to_string()));
        assert_eq!(captured[1], (42, "test log line 2".to_string()));
        assert_eq!(captured[2], (99, "another log".to_string()));
    }

    #[tokio::test]
    async fn test_current_download_id() {
        let _guard = TEST_MUTEX.lock().unwrap();

        assert_eq!(current_download_id(), None);

        CURRENT_DOWNLOAD_ID
            .scope(123, async {
                assert_eq!(current_download_id(), Some(123));

                CURRENT_DOWNLOAD_ID
                    .scope(456, async {
                        assert_eq!(current_download_id(), Some(456));
                    })
                    .await;

                assert_eq!(current_download_id(), Some(123));
            })
            .await;

        assert_eq!(current_download_id(), None);
    }
}
