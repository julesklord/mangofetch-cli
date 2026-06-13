use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub min_mangofetch_version: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub nav: Vec<PluginNavItem>,
    #[serde(default)]
    pub events: PluginEvents,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub settings_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub rust_crate: Option<String>,
    #[serde(default)]
    pub frontend_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginNavItem {
    pub route: String,
    pub label: HashMap<String, String>,
    #[serde(default)]
    pub icon_svg: Option<String>,
    #[serde(default = "default_nav_group")]
    pub group: NavGroup,
    #[serde(default = "default_nav_order")]
    pub order: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginEvents {
    #[serde(default)]
    pub progress: Vec<String>,
    #[serde(default)]
    pub complete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NavGroup {
    Primary,
    #[default]
    Secondary,
}

fn default_nav_group() -> NavGroup {
    NavGroup::Secondary
}

fn default_nav_order() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub version: String,
    pub installed_at: String,
    pub updated_at: String,
    pub enabled: bool,
    pub repo: Option<String>,
    pub source_release: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub repo: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    #[serde(default)]
    pub schema_version: u32,
    pub plugins: Vec<RegistryEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest_default_deserialization() {
        let json = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "author": "Test Author"
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();

        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.min_mangofetch_version, None);
        assert_eq!(manifest.license, None);
        assert_eq!(manifest.homepage, None);
        assert_eq!(manifest.icon, None);
        assert!(manifest.nav.is_empty());
        assert!(manifest.events.progress.is_empty());
        assert!(manifest.events.complete.is_empty());
        assert!(manifest.capabilities.is_empty());
        assert_eq!(manifest.settings_schema, None);
        assert_eq!(manifest.rust_crate, None);
        assert_eq!(manifest.frontend_dir, None);
    }

    #[test]
    fn test_plugin_nav_item_default_deserialization() {
        let json = r#"{
            "route": "/test",
            "label": {
                "en": "Test"
            }
        }"#;

        let nav_item: PluginNavItem = serde_json::from_str(json).unwrap();

        assert_eq!(nav_item.route, "/test");
        assert_eq!(nav_item.icon_svg, None);
        assert_eq!(nav_item.group, NavGroup::Secondary);
        assert_eq!(nav_item.order, 50);
    }

    #[test]
    fn test_registry_entry_default_deserialization() {
        let json = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "description": "A test plugin",
            "author": "Test Author",
            "repo": "https://github.com/test/repo"
        }"#;

        let entry: RegistryEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.id, "test-plugin");
        assert_eq!(entry.homepage, None);
        assert!(entry.tags.is_empty());
        assert_eq!(entry.official, false);
        assert!(entry.capabilities.is_empty());
    }
}
