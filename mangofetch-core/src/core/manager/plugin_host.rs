use mangofetch_plugin_sdk::{PluginHost, ProxyConfig};
use std::path::PathBuf;

pub struct CorePluginHost;

impl PluginHost for CorePluginHost {
    fn emit_event(&self, name: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        tracing::info!("[plugin-event] {}: {}", name, payload);
        Ok(())
    }

    fn show_toast(&self, toast_type: &str, message: &str) -> anyhow::Result<()> {
        tracing::info!("[plugin-toast] [{}]: {}", toast_type, message);
        Ok(())
    }

    fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf {
        crate::core::paths::app_data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("plugins")
            .join(plugin_id)
    }

    fn plugin_frontend_dir(&self, plugin_id: &str) -> PathBuf {
        crate::core::paths::app_data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("plugins")
            .join(plugin_id)
            .join("frontend")
    }

    fn get_settings(&self, _plugin_id: &str) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn save_settings(&self, _plugin_id: &str, _settings: serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }

    fn proxy_config(&self) -> Option<ProxyConfig> {
        None
    }

    fn tool_path(&self, _tool: &str) -> Option<PathBuf> {
        None
    }

    fn default_output_dir(&self) -> PathBuf {
        dirs::download_dir().unwrap_or_else(|| PathBuf::from("."))
    }
}
