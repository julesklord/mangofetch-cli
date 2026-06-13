use crate::core::manager::plugin_host::CorePluginHost;
use anyhow::Context;
use libloading::{Library, Symbol};
use mangofetch_plugin_sdk::MangoFetchPlugin;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[allow(improper_ctypes_definitions)]
type PluginInitFn = unsafe extern "C" fn() -> *mut dyn MangoFetchPlugin;

pub struct PluginInstance {
    pub plugin: Box<dyn MangoFetchPlugin>,
    _library: Library, // Keep library in memory
}

pub struct PluginManager {
    plugins: HashMap<String, PluginInstance>,
    host: Arc<CorePluginHost>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            host: Arc::new(CorePluginHost),
        }
    }

    pub async fn load_plugins(&mut self) -> anyhow::Result<()> {
        let plugins_dir = crate::core::paths::app_data_dir()
            .context("Failed to get app data directory")?
            .join("plugins");

        if !plugins_dir.exists() {
            std::fs::create_dir_all(&plugins_dir)?;
            return Ok(());
        }

        for entry in std::fs::read_dir(plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str == "dll" || ext_str == "so" || ext_str == "dylib" {
                        if let Err(e) = self.load_single_plugin(path.clone()) {
                            tracing::error!("Failed to load plugin from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn load_single_plugin(&mut self, path: PathBuf) -> anyhow::Result<()> {
        unsafe {
            let lib = Library::new(&path).context("Failed to load dynamic library")?;

            let init_fn: Symbol<PluginInitFn> = lib
                .get(b"mangofetch_plugin_init")
                .context("Failed to find mangofetch_plugin_init symbol")?;

            let plugin_ptr = init_fn();
            let mut plugin = Box::from_raw(plugin_ptr);

            plugin
                .initialize(self.host.clone())
                .context("Failed to initialize plugin")?;

            let id = plugin.id().to_string();
            tracing::info!("Loaded plugin: {} ({}) from {:?}", plugin.name(), id, path);

            self.plugins.insert(
                id,
                PluginInstance {
                    plugin,
                    _library: lib,
                },
            );
        }

        Ok(())
    }

    pub fn get_plugin(&self, id: &str) -> Option<&dyn MangoFetchPlugin> {
        self.plugins.get(id).map(|p| p.plugin.as_ref())
    }

    pub fn list_plugins(&self) -> Vec<(&String, &str)> {
        self.plugins
            .iter()
            .map(|(id, p)| (id, p.plugin.name()))
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
