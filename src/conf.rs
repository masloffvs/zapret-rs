use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conf {
  pub interface: String,
  pub auto_update: bool,
  pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfResult {
  pub is_fallback: bool,
  pub conf: Conf,
}

impl Conf {
  pub fn new(interface: String, auto_update: bool, strategy: String) -> Self {
    Self { interface, auto_update, strategy }
  }

  pub fn load(file_path: &str) -> ConfResult {
    println!("[DEBUG] Conf::load - file_path: {}", file_path);
    
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Warning: Failed to create directory {}: {}", parent.display(), e);
            }
        }
    }
    
    if !fs::metadata(file_path).is_ok() {
      println!("[DEBUG] Conf::load - file not found, using fallback");
      return ConfResult { is_fallback: true, conf: Self::new("".to_string(), false, "".to_string()) };
    }

    println!("[DEBUG] Conf::load - file found, reading content");
    let conf = fs::read_to_string(file_path).unwrap();
    println!("[DEBUG] Conf::load - content: {}", conf);
    let conf: Conf = serde_json::from_str(&conf).unwrap();
    println!("[DEBUG] Conf::load - parsed successfully");
    ConfResult { is_fallback: false, conf }
  }

  pub fn save(self, file_path: &str) {
    let conf = serde_json::to_string(&self).unwrap();
    fs::write(file_path, conf).unwrap();
  }
}