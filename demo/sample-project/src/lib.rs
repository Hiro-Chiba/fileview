pub mod utils;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub version: String,
    pub debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "sample-project".to_string(),
            version: "0.1.0".to_string(),
            debug: false,
        }
    }
}

pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.name, "sample-project");
        assert!(!config.debug);
    }
}
