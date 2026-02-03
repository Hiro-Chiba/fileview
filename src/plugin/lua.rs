//! Lua runtime management for plugin system

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mlua::Lua;

use super::api::PluginContext;

/// Plugin system error
#[derive(Debug)]
pub enum PluginError {
    /// Failed to create Lua runtime
    RuntimeError(String),
    /// Failed to load plugin file
    LoadError(String),
    /// Plugin execution error
    ExecutionError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::RuntimeError(msg) => write!(f, "Plugin runtime error: {}", msg),
            PluginError::LoadError(msg) => write!(f, "Plugin load error: {}", msg),
            PluginError::ExecutionError(msg) => write!(f, "Plugin execution error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<mlua::Error> for PluginError {
    fn from(err: mlua::Error) -> Self {
        PluginError::ExecutionError(err.to_string())
    }
}

/// Manages the Lua plugin runtime
pub struct PluginManager {
    /// Lua runtime instance
    lua: Lua,
    /// Shared plugin context
    context: Arc<Mutex<PluginContext>>,
    /// Whether plugins are loaded
    loaded: bool,
    /// Notifications to display (queued from plugins)
    pending_notifications: Vec<String>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Result<Self, PluginError> {
        let lua = Lua::new();
        let context = Arc::new(Mutex::new(PluginContext::new()));

        // Set up the 'fv' global table with API functions
        Self::setup_api(&lua, Arc::clone(&context))?;

        Ok(Self {
            lua,
            context,
            loaded: false,
            pending_notifications: Vec::new(),
        })
    }

    /// Set up the 'fv' API table in Lua
    fn setup_api(lua: &Lua, context: Arc<Mutex<PluginContext>>) -> Result<(), PluginError> {
        let globals = lua.globals();

        // Create 'fv' table
        let fv = lua.create_table().map_err(PluginError::from)?;

        // fv.current_file() -> string or nil
        {
            let ctx = Arc::clone(&context);
            let current_file = lua
                .create_function(move |_, ()| {
                    let ctx = ctx.lock().unwrap();
                    Ok(ctx.current_file().map(|p| p.to_string_lossy().to_string()))
                })
                .map_err(PluginError::from)?;
            fv.set("current_file", current_file)
                .map_err(PluginError::from)?;
        }

        // fv.current_dir() -> string
        {
            let ctx = Arc::clone(&context);
            let current_dir = lua
                .create_function(move |_, ()| {
                    let ctx = ctx.lock().unwrap();
                    Ok(ctx.current_dir().to_string_lossy().to_string())
                })
                .map_err(PluginError::from)?;
            fv.set("current_dir", current_dir)
                .map_err(PluginError::from)?;
        }

        // fv.selected_files() -> table (array of strings)
        {
            let ctx = Arc::clone(&context);
            let selected_files = lua
                .create_function(move |lua, ()| {
                    let ctx = ctx.lock().unwrap();
                    let files = ctx.selected_files();
                    let table = lua.create_table()?;
                    for (i, path) in files.iter().enumerate() {
                        table.set(i + 1, path.to_string_lossy().to_string())?;
                    }
                    Ok(table)
                })
                .map_err(PluginError::from)?;
            fv.set("selected_files", selected_files)
                .map_err(PluginError::from)?;
        }

        // fv.notify(message) -> nil
        {
            let ctx = Arc::clone(&context);
            let notify = lua
                .create_function(move |_, msg: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.add_notification(msg);
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("notify", notify).map_err(PluginError::from)?;
        }

        // fv.version() -> string
        {
            let version = lua
                .create_function(|_, ()| Ok(env!("CARGO_PKG_VERSION").to_string()))
                .map_err(PluginError::from)?;
            fv.set("version", version).map_err(PluginError::from)?;
        }

        // fv.is_dir(path) -> boolean
        {
            let is_dir = lua
                .create_function(|_, path: String| Ok(Path::new(&path).is_dir()))
                .map_err(PluginError::from)?;
            fv.set("is_dir", is_dir).map_err(PluginError::from)?;
        }

        // fv.file_exists(path) -> boolean
        {
            let file_exists = lua
                .create_function(|_, path: String| Ok(Path::new(&path).exists()))
                .map_err(PluginError::from)?;
            fv.set("file_exists", file_exists)
                .map_err(PluginError::from)?;
        }

        // Set the fv table as a global
        globals.set("fv", fv).map_err(PluginError::from)?;

        Ok(())
    }

    /// Get the plugin directory path
    pub fn plugin_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("fileview").join("plugins"))
    }

    /// Get the init.lua path
    pub fn init_lua_path() -> Option<PathBuf> {
        Self::plugin_dir().map(|p| p.join("init.lua"))
    }

    /// Load plugins from the plugin directory
    pub fn load_plugins(&mut self) -> Result<(), PluginError> {
        if self.loaded {
            return Ok(());
        }

        let init_path = match Self::init_lua_path() {
            Some(p) if p.exists() => p,
            _ => {
                // No init.lua found, that's fine
                self.loaded = true;
                return Ok(());
            }
        };

        // Read and execute init.lua
        let code = std::fs::read_to_string(&init_path).map_err(|e| {
            PluginError::LoadError(format!("Failed to read {}: {}", init_path.display(), e))
        })?;

        self.lua
            .load(&code)
            .set_name(init_path.to_string_lossy())
            .exec()
            .map_err(|e| {
                PluginError::ExecutionError(format!("Error in {}: {}", init_path.display(), e))
            })?;

        // Collect notifications
        self.collect_notifications();

        self.loaded = true;
        Ok(())
    }

    /// Update the plugin context with current state
    pub fn update_context(
        &mut self,
        current_file: Option<PathBuf>,
        current_dir: PathBuf,
        selected_files: Vec<PathBuf>,
    ) {
        let mut ctx = self.context.lock().unwrap();
        ctx.set_current_file(current_file);
        ctx.set_current_dir(current_dir);
        ctx.set_selected_files(selected_files);
    }

    /// Collect pending notifications from the context
    fn collect_notifications(&mut self) {
        let mut ctx = self.context.lock().unwrap();
        self.pending_notifications
            .extend(ctx.take_notifications());
    }

    /// Take pending notifications
    pub fn take_notifications(&mut self) -> Vec<String> {
        self.collect_notifications();
        std::mem::take(&mut self.pending_notifications)
    }

    /// Execute a Lua string (for testing or REPL)
    pub fn exec(&mut self, code: &str) -> Result<(), PluginError> {
        self.lua.load(code).exec().map_err(PluginError::from)?;
        self.collect_notifications();
        Ok(())
    }

    /// Evaluate a Lua expression and return the result as a string
    pub fn eval(&self, code: &str) -> Result<String, PluginError> {
        let result: mlua::Value = self.lua.load(code).eval().map_err(PluginError::from)?;
        Ok(format_lua_value(&result))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin manager")
    }
}

/// Format a Lua value as a string for display
fn format_lua_value(value: &mlua::Value) -> String {
    match value {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => s
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<invalid utf8>".to_string()),
        mlua::Value::Table(_) => "<table>".to_string(),
        mlua::Value::Function(_) => "<function>".to_string(),
        mlua::Value::Thread(_) => "<thread>".to_string(),
        mlua::Value::LightUserData(_) => "<userdata>".to_string(),
        mlua::Value::UserData(_) => "<userdata>".to_string(),
        mlua::Value::Error(e) => format!("<error: {}>", e),
        _ => "<unknown>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let manager = PluginManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_fv_version() {
        let manager = PluginManager::new().unwrap();
        let result = manager.eval("fv.version()");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("1.")); // Should contain version like "1.19.0"
    }

    #[test]
    fn test_fv_current_dir() {
        let mut manager = PluginManager::new().unwrap();
        manager.update_context(None, PathBuf::from("/test/dir"), vec![]);

        let result = manager.eval("fv.current_dir()");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/test/dir");
    }

    #[test]
    fn test_fv_current_file() {
        let mut manager = PluginManager::new().unwrap();
        manager.update_context(
            Some(PathBuf::from("/test/file.txt")),
            PathBuf::from("/test"),
            vec![],
        );

        let result = manager.eval("fv.current_file()");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/test/file.txt");
    }

    #[test]
    fn test_fv_current_file_nil() {
        let mut manager = PluginManager::new().unwrap();
        manager.update_context(None, PathBuf::from("/test"), vec![]);

        let result = manager.eval("fv.current_file()");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "nil");
    }

    #[test]
    fn test_fv_selected_files() {
        let mut manager = PluginManager::new().unwrap();
        manager.update_context(
            None,
            PathBuf::from("/test"),
            vec![PathBuf::from("/test/a.txt"), PathBuf::from("/test/b.txt")],
        );

        let result = manager.eval("#fv.selected_files()");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");
    }

    #[test]
    fn test_fv_notify() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.notify('Hello from Lua!')").unwrap();

        let notifications = manager.take_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0], "Hello from Lua!");
    }

    #[test]
    fn test_fv_is_dir() {
        let manager = PluginManager::new().unwrap();

        // Test with a directory that definitely exists
        let result = manager.eval("fv.is_dir('/')");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "true");

        // Test with a file that doesn't exist
        let result = manager.eval("fv.is_dir('/nonexistent/path')");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "false");
    }

    #[test]
    fn test_fv_file_exists() {
        let manager = PluginManager::new().unwrap();

        // Test with root directory
        let result = manager.eval("fv.file_exists('/')");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "true");

        // Test with nonexistent path
        let result = manager.eval("fv.file_exists('/definitely/does/not/exist/xyz123')");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "false");
    }

    #[test]
    fn test_exec_lua_code() {
        let mut manager = PluginManager::new().unwrap();
        let result = manager.exec("local x = 1 + 1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_exec_lua_error() {
        let mut manager = PluginManager::new().unwrap();
        let result = manager.exec("this is not valid lua");
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_dir() {
        let dir = PluginManager::plugin_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.to_string_lossy().contains("fileview"));
        assert!(dir.to_string_lossy().contains("plugins"));
    }

    #[test]
    fn test_multiple_notifications() {
        let mut manager = PluginManager::new().unwrap();
        manager
            .exec(
                r#"
            fv.notify("First")
            fv.notify("Second")
            fv.notify("Third")
        "#,
            )
            .unwrap();

        let notifications = manager.take_notifications();
        assert_eq!(notifications.len(), 3);
        assert_eq!(notifications[0], "First");
        assert_eq!(notifications[1], "Second");
        assert_eq!(notifications[2], "Third");
    }
}
