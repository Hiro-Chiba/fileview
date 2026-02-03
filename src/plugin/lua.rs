//! Lua runtime management for plugin system

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mlua::{Function, Lua};

use super::api::{PluginAction, PluginContext, PluginEvent};

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
    /// Actions to execute (queued from plugins)
    pending_actions: Vec<PluginAction>,
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
            pending_actions: Vec::new(),
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

        // === Action API (Phase 12b) ===

        // fv.navigate(path) -> nil
        // Navigate to a directory
        {
            let ctx = Arc::clone(&context);
            let navigate = lua
                .create_function(move |_, path: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::Navigate(PathBuf::from(path)));
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("navigate", navigate).map_err(PluginError::from)?;
        }

        // fv.select(path) -> nil
        // Add a file to selection
        {
            let ctx = Arc::clone(&context);
            let select = lua
                .create_function(move |_, path: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::Select(PathBuf::from(path)));
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("select", select).map_err(PluginError::from)?;
        }

        // fv.deselect(path) -> nil
        // Remove a file from selection
        {
            let ctx = Arc::clone(&context);
            let deselect = lua
                .create_function(move |_, path: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::Deselect(PathBuf::from(path)));
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("deselect", deselect).map_err(PluginError::from)?;
        }

        // fv.clear_selection() -> nil
        // Clear all selections
        {
            let ctx = Arc::clone(&context);
            let clear_selection = lua
                .create_function(move |_, ()| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::ClearSelection);
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("clear_selection", clear_selection)
                .map_err(PluginError::from)?;
        }

        // fv.refresh() -> nil
        // Refresh the tree view
        {
            let ctx = Arc::clone(&context);
            let refresh = lua
                .create_function(move |_, ()| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::Refresh);
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("refresh", refresh).map_err(PluginError::from)?;
        }

        // fv.set_clipboard(text) -> nil
        // Set clipboard text
        {
            let ctx = Arc::clone(&context);
            let set_clipboard = lua
                .create_function(move |_, text: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::SetClipboard(text));
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("set_clipboard", set_clipboard)
                .map_err(PluginError::from)?;
        }

        // fv.focus(path) -> nil
        // Focus on a specific file (reveal and select)
        {
            let ctx = Arc::clone(&context);
            let focus = lua
                .create_function(move |_, path: String| {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.queue_action(PluginAction::Focus(PathBuf::from(path)));
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("focus", focus).map_err(PluginError::from)?;
        }

        // === Registration API (Phase 12c) ===

        // Internal storage tables
        let commands_table = lua.create_table().map_err(PluginError::from)?;
        let events_table = lua.create_table().map_err(PluginError::from)?;
        let previewers_table = lua.create_table().map_err(PluginError::from)?;

        fv.set("_commands", commands_table)
            .map_err(PluginError::from)?;
        fv.set("_events", events_table).map_err(PluginError::from)?;
        fv.set("_previewers", previewers_table)
            .map_err(PluginError::from)?;

        // fv.register_command(name, fn) -> nil
        // Register a custom command
        {
            let register_command = lua
                .create_function(|lua, (name, func): (String, Function)| {
                    let globals = lua.globals();
                    let fv: mlua::Table = globals.get("fv")?;
                    let commands: mlua::Table = fv.get("_commands")?;
                    commands.set(name, func)?;
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("register_command", register_command)
                .map_err(PluginError::from)?;
        }

        // fv.on(event, fn) -> nil
        // Register an event handler
        {
            let on = lua
                .create_function(|lua, (event, func): (String, Function)| {
                    let globals = lua.globals();
                    let fv: mlua::Table = globals.get("fv")?;
                    let events: mlua::Table = fv.get("_events")?;

                    // Get or create the handler list for this event
                    let handlers: mlua::Table = match events.get::<mlua::Table>(event.clone()) {
                        Ok(t) => t,
                        Err(_) => {
                            let new_table = lua.create_table()?;
                            events.set(event.clone(), new_table.clone())?;
                            new_table
                        }
                    };

                    // Append the handler
                    let len = handlers.len()?;
                    handlers.set(len + 1, func)?;

                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("on", on).map_err(PluginError::from)?;
        }

        // fv.register_previewer(pattern, fn) -> nil
        // Register a custom previewer for a file pattern
        {
            let register_previewer = lua
                .create_function(|lua, (pattern, func): (String, Function)| {
                    let globals = lua.globals();
                    let fv: mlua::Table = globals.get("fv")?;
                    let previewers: mlua::Table = fv.get("_previewers")?;
                    previewers.set(pattern, func)?;
                    Ok(())
                })
                .map_err(PluginError::from)?;
            fv.set("register_previewer", register_previewer)
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
        self.pending_notifications.extend(ctx.take_notifications());
    }

    /// Take pending notifications
    pub fn take_notifications(&mut self) -> Vec<String> {
        self.collect_notifications();
        std::mem::take(&mut self.pending_notifications)
    }

    /// Collect pending actions from the context
    fn collect_actions(&mut self) {
        let mut ctx = self.context.lock().unwrap();
        self.pending_actions.extend(ctx.take_actions());
    }

    /// Take pending actions
    pub fn take_actions(&mut self) -> Vec<PluginAction> {
        self.collect_actions();
        std::mem::take(&mut self.pending_actions)
    }

    /// Execute a Lua string (for testing or REPL)
    pub fn exec(&mut self, code: &str) -> Result<(), PluginError> {
        self.lua.load(code).exec().map_err(PluginError::from)?;
        self.collect_notifications();
        self.collect_actions();
        Ok(())
    }

    /// Evaluate a Lua expression and return the result as a string
    pub fn eval(&self, code: &str) -> Result<String, PluginError> {
        let result: mlua::Value = self.lua.load(code).eval().map_err(PluginError::from)?;
        Ok(format_lua_value(&result))
    }

    // === Phase 12c: Registration API methods ===

    /// Check if a command is registered
    pub fn has_command(&self, name: &str) -> bool {
        let Ok(globals) = self.lua.globals().get::<mlua::Table>("fv") else {
            return false;
        };
        let Ok(commands) = globals.get::<mlua::Table>("_commands") else {
            return false;
        };
        commands.contains_key(name).unwrap_or(false)
    }

    /// Get list of registered command names
    pub fn list_commands(&self) -> Vec<String> {
        let mut names = Vec::new();
        let Ok(globals) = self.lua.globals().get::<mlua::Table>("fv") else {
            return names;
        };
        let Ok(commands) = globals.get::<mlua::Table>("_commands") else {
            return names;
        };
        for (name, _) in commands.pairs::<String, mlua::Value>().flatten() {
            names.push(name);
        }
        names
    }

    /// Invoke a registered command by name
    pub fn invoke_command(&mut self, name: &str) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        let fv: mlua::Table = globals.get("fv").map_err(PluginError::from)?;
        let commands: mlua::Table = fv.get("_commands").map_err(PluginError::from)?;

        let func: Function = commands
            .get(name)
            .map_err(|_| PluginError::ExecutionError(format!("Command '{}' not found", name)))?;

        func.call::<()>(()).map_err(|e| {
            PluginError::ExecutionError(format!("Error in command '{}': {}", name, e))
        })?;

        self.collect_notifications();
        self.collect_actions();
        Ok(())
    }

    /// Fire an event and call all registered handlers
    pub fn fire_event(&mut self, event: PluginEvent, arg: Option<&str>) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        let fv: mlua::Table = globals.get("fv").map_err(PluginError::from)?;
        let events: mlua::Table = fv.get("_events").map_err(PluginError::from)?;

        let event_name = event.as_str();
        let handlers: mlua::Table = match events.get(event_name) {
            Ok(t) => t,
            Err(_) => return Ok(()), // No handlers registered
        };

        // Call each handler
        for (_, func) in handlers.pairs::<i64, Function>().flatten() {
            if let Some(arg_str) = arg {
                let _ = func.call::<()>(arg_str);
            } else {
                let _ = func.call::<()>(());
            }
        }

        self.collect_notifications();
        self.collect_actions();
        Ok(())
    }

    /// Check if a previewer is registered for a pattern
    pub fn has_previewer(&self, pattern: &str) -> bool {
        let Ok(globals) = self.lua.globals().get::<mlua::Table>("fv") else {
            return false;
        };
        let Ok(previewers) = globals.get::<mlua::Table>("_previewers") else {
            return false;
        };
        previewers.contains_key(pattern).unwrap_or(false)
    }

    /// Get list of registered previewer patterns
    pub fn list_previewers(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        let Ok(globals) = self.lua.globals().get::<mlua::Table>("fv") else {
            return patterns;
        };
        let Ok(previewers) = globals.get::<mlua::Table>("_previewers") else {
            return patterns;
        };
        for (pattern, _) in previewers.pairs::<String, mlua::Value>().flatten() {
            patterns.push(pattern);
        }
        patterns
    }

    /// Find a previewer that matches the given filename
    /// Returns the pattern that matched
    pub fn find_previewer(&self, filename: &str) -> Option<String> {
        self.list_previewers()
            .into_iter()
            .find(|pattern| matches_glob(pattern, filename))
    }

    /// Invoke a previewer and get the result
    /// Returns the preview content as a string
    pub fn invoke_previewer(&mut self, pattern: &str, path: &str) -> Result<String, PluginError> {
        let globals = self.lua.globals();
        let fv: mlua::Table = globals.get("fv").map_err(PluginError::from)?;
        let previewers: mlua::Table = fv.get("_previewers").map_err(PluginError::from)?;

        let func: Function = previewers.get(pattern).map_err(|_| {
            PluginError::ExecutionError(format!("Previewer '{}' not found", pattern))
        })?;

        let result: String = func.call(path).map_err(|e| {
            PluginError::ExecutionError(format!("Error in previewer '{}': {}", pattern, e))
        })?;

        self.collect_notifications();
        self.collect_actions();
        Ok(result)
    }
}

/// Simple glob pattern matching (supports * and ?)
fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    matches_glob_recursive(&pattern_chars, &text_chars, 0, 0)
}

fn matches_glob_recursive(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    if pi == pattern.len() {
        return ti == text.len();
    }

    match pattern[pi] {
        '*' => {
            // Match zero or more characters
            for i in ti..=text.len() {
                if matches_glob_recursive(pattern, text, pi + 1, i) {
                    return true;
                }
            }
            false
        }
        '?' => {
            // Match exactly one character
            if ti < text.len() {
                matches_glob_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
        c => {
            // Match exact character
            if ti < text.len() && text[ti] == c {
                matches_glob_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
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

    // === Phase 12b: Action API Tests ===

    #[test]
    fn test_fv_navigate() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.navigate('/home/user')").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::Navigate(PathBuf::from("/home/user"))
        );
    }

    #[test]
    fn test_fv_select() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.select('/test/file.txt')").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::Select(PathBuf::from("/test/file.txt"))
        );
    }

    #[test]
    fn test_fv_deselect() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.deselect('/test/file.txt')").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::Deselect(PathBuf::from("/test/file.txt"))
        );
    }

    #[test]
    fn test_fv_clear_selection() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.clear_selection()").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], PluginAction::ClearSelection);
    }

    #[test]
    fn test_fv_refresh() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.refresh()").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], PluginAction::Refresh);
    }

    #[test]
    fn test_fv_set_clipboard() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.set_clipboard('copied text')").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::SetClipboard("copied text".to_string())
        );
    }

    #[test]
    fn test_fv_focus() {
        let mut manager = PluginManager::new().unwrap();
        manager.exec("fv.focus('/test/target.txt')").unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            PluginAction::Focus(PathBuf::from("/test/target.txt"))
        );
    }

    #[test]
    fn test_multiple_actions() {
        let mut manager = PluginManager::new().unwrap();
        manager
            .exec(
                r#"
            fv.navigate("/home")
            fv.select("/home/file1.txt")
            fv.select("/home/file2.txt")
            fv.refresh()
        "#,
            )
            .unwrap();

        let actions = manager.take_actions();
        assert_eq!(actions.len(), 4);
        assert_eq!(actions[0], PluginAction::Navigate(PathBuf::from("/home")));
        assert_eq!(
            actions[1],
            PluginAction::Select(PathBuf::from("/home/file1.txt"))
        );
        assert_eq!(
            actions[2],
            PluginAction::Select(PathBuf::from("/home/file2.txt"))
        );
        assert_eq!(actions[3], PluginAction::Refresh);
    }

    #[test]
    fn test_actions_and_notifications_combined() {
        let mut manager = PluginManager::new().unwrap();
        manager
            .exec(
                r#"
            fv.notify("Starting...")
            fv.navigate("/test")
            fv.notify("Done!")
        "#,
            )
            .unwrap();

        let notifications = manager.take_notifications();
        let actions = manager.take_actions();

        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0], "Starting...");
        assert_eq!(notifications[1], "Done!");

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], PluginAction::Navigate(PathBuf::from("/test")));
    }

    // === Phase 12c: Registration API Tests ===

    #[test]
    fn test_register_command() {
        let mut manager = PluginManager::new().unwrap();

        // Register a command
        manager
            .exec(
                r#"
            fv.register_command("test-cmd", function()
                fv.notify("Command executed!")
            end)
        "#,
            )
            .unwrap();

        // Check it's registered
        assert!(manager.has_command("test-cmd"));
        assert!(!manager.has_command("nonexistent"));
    }

    #[test]
    fn test_invoke_command() {
        let mut manager = PluginManager::new().unwrap();

        // Register a command that sets a notification
        manager
            .exec(
                r#"
            fv.register_command("hello", function()
                fv.notify("Hello from command!")
            end)
        "#,
            )
            .unwrap();

        // Invoke it
        manager.invoke_command("hello").unwrap();

        let notifications = manager.take_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0], "Hello from command!");
    }

    #[test]
    fn test_invoke_command_not_found() {
        let mut manager = PluginManager::new().unwrap();

        let result = manager.invoke_command("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_commands() {
        let mut manager = PluginManager::new().unwrap();

        manager
            .exec(
                r#"
            fv.register_command("cmd-a", function() end)
            fv.register_command("cmd-b", function() end)
        "#,
            )
            .unwrap();

        let commands = manager.list_commands();
        assert_eq!(commands.len(), 2);
        assert!(commands.contains(&"cmd-a".to_string()));
        assert!(commands.contains(&"cmd-b".to_string()));
    }

    #[test]
    fn test_on_event() {
        let mut manager = PluginManager::new().unwrap();

        // Register an event handler
        manager
            .exec(
                r#"
            fv.on("file_selected", function(path)
                fv.notify("Selected: " .. (path or "nil"))
            end)
        "#,
            )
            .unwrap();

        // Fire the event
        manager
            .fire_event(PluginEvent::FileSelected, Some("/test/file.txt"))
            .unwrap();

        let notifications = manager.take_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0], "Selected: /test/file.txt");
    }

    #[test]
    fn test_multiple_event_handlers() {
        let mut manager = PluginManager::new().unwrap();

        // Register multiple handlers for the same event
        manager
            .exec(
                r#"
            fv.on("start", function()
                fv.notify("Handler 1")
            end)
            fv.on("start", function()
                fv.notify("Handler 2")
            end)
        "#,
            )
            .unwrap();

        // Fire the event
        manager.fire_event(PluginEvent::Start, None).unwrap();

        let notifications = manager.take_notifications();
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0], "Handler 1");
        assert_eq!(notifications[1], "Handler 2");
    }

    #[test]
    fn test_fire_event_no_handlers() {
        let mut manager = PluginManager::new().unwrap();

        // Should not error when no handlers registered
        let result = manager.fire_event(PluginEvent::BeforeQuit, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_previewer() {
        let mut manager = PluginManager::new().unwrap();

        manager
            .exec(
                r#"
            fv.register_previewer("*.csv", function(path)
                return "CSV preview for: " .. path
            end)
        "#,
            )
            .unwrap();

        assert!(manager.has_previewer("*.csv"));
        assert!(!manager.has_previewer("*.json"));
    }

    #[test]
    fn test_invoke_previewer() {
        let mut manager = PluginManager::new().unwrap();

        manager
            .exec(
                r#"
            fv.register_previewer("*.md", function(path)
                return "Markdown: " .. path
            end)
        "#,
            )
            .unwrap();

        let result = manager.invoke_previewer("*.md", "/test/README.md");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Markdown: /test/README.md");
    }

    #[test]
    fn test_find_previewer() {
        let mut manager = PluginManager::new().unwrap();

        manager
            .exec(
                r#"
            fv.register_previewer("*.rs", function(path) return "Rust" end)
            fv.register_previewer("*.py", function(path) return "Python" end)
        "#,
            )
            .unwrap();

        assert_eq!(manager.find_previewer("main.rs"), Some("*.rs".to_string()));
        assert_eq!(
            manager.find_previewer("script.py"),
            Some("*.py".to_string())
        );
        assert_eq!(manager.find_previewer("file.txt"), None);
    }

    #[test]
    fn test_list_previewers() {
        let mut manager = PluginManager::new().unwrap();

        manager
            .exec(
                r#"
            fv.register_previewer("*.json", function(path) return "{}" end)
            fv.register_previewer("*.yaml", function(path) return "---" end)
        "#,
            )
            .unwrap();

        let previewers = manager.list_previewers();
        assert_eq!(previewers.len(), 2);
        assert!(previewers.contains(&"*.json".to_string()));
        assert!(previewers.contains(&"*.yaml".to_string()));
    }

    // === Glob matching tests ===

    #[test]
    fn test_matches_glob_exact() {
        assert!(matches_glob("test.txt", "test.txt"));
        assert!(!matches_glob("test.txt", "test.rs"));
    }

    #[test]
    fn test_matches_glob_star() {
        assert!(matches_glob("*.txt", "file.txt"));
        assert!(matches_glob("*.txt", "another.txt"));
        assert!(!matches_glob("*.txt", "file.rs"));
        assert!(matches_glob("test*", "test.txt"));
        assert!(matches_glob("test*", "testing"));
        assert!(matches_glob("*test*", "my_test_file"));
    }

    #[test]
    fn test_matches_glob_question() {
        assert!(matches_glob("file?.txt", "file1.txt"));
        assert!(matches_glob("file?.txt", "fileA.txt"));
        assert!(!matches_glob("file?.txt", "file12.txt"));
    }

    #[test]
    fn test_matches_glob_combined() {
        assert!(matches_glob("*.??", "file.rs"));
        assert!(matches_glob("*.??", "test.py"));
        assert!(!matches_glob("*.??", "file.txt"));
    }

    // === PluginEvent tests ===

    #[test]
    fn test_plugin_event_as_str() {
        assert_eq!(PluginEvent::FileSelected.as_str(), "file_selected");
        assert_eq!(PluginEvent::DirectoryChanged.as_str(), "directory_changed");
        assert_eq!(PluginEvent::Start.as_str(), "start");
    }

    #[test]
    fn test_plugin_event_parse() {
        assert_eq!(
            PluginEvent::parse("file_selected"),
            Some(PluginEvent::FileSelected)
        );
        assert_eq!(PluginEvent::parse("invalid"), None);
    }
}
