use std::path::PathBuf;

pub trait PluginHost: Send + Sync {
    fn emit_event(&self, name: &str, payload: serde_json::Value) -> anyhow::Result<()>;
    fn show_toast(&self, toast_type: &str, message: &str) -> anyhow::Result<()>;
    fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf;
    fn plugin_frontend_dir(&self, plugin_id: &str) -> PathBuf;
    fn get_settings(&self, plugin_id: &str) -> serde_json::Value;
    fn save_settings(&self, plugin_id: &str, settings: serde_json::Value) -> anyhow::Result<()>;
    fn proxy_config(&self) -> Option<ProxyConfig>;
    fn tool_path(&self, tool: &str) -> Option<PathBuf>;
    fn default_output_dir(&self) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_creation() {
        let config = ProxyConfig {
            proxy_type: "http".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            username: None,
            password: None,
        };

        assert_eq!(config.proxy_type, "http");
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.username, None);
        assert_eq!(config.password, None);
    }

    #[test]
    fn test_proxy_config_with_auth() {
        let config = ProxyConfig {
            proxy_type: "socks5".to_string(),
            host: "proxy.example.com".to_string(),
            port: 1080,
            username: Some("user123".to_string()),
            password: Some("secret_pass".to_string()),
        };

        assert_eq!(config.proxy_type, "socks5");
        assert_eq!(config.host, "proxy.example.com");
        assert_eq!(config.port, 1080);
        assert_eq!(config.username.as_deref(), Some("user123"));
        assert_eq!(config.password.as_deref(), Some("secret_pass"));
    }

    #[test]
    fn test_proxy_config_clone() {
        let config = ProxyConfig {
            proxy_type: "http".to_string(),
            host: "localhost".to_string(),
            port: 3128,
            username: Some("testuser".to_string()),
            password: None,
        };

        let cloned = config.clone();

        assert_eq!(config.proxy_type, cloned.proxy_type);
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.username, cloned.username);
        assert_eq!(config.password, cloned.password);
    }

    #[test]
    fn test_proxy_config_debug() {
        let config = ProxyConfig {
            proxy_type: "https".to_string(),
            host: "secure.proxy.org".to_string(),
            port: 443,
            username: None,
            password: None,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ProxyConfig"));
        assert!(debug_str.contains("proxy_type: \"https\""));
        assert!(debug_str.contains("host: \"secure.proxy.org\""));
        assert!(debug_str.contains("port: 443"));
    }
}
