use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMeta {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub name: String,
    pub repo_name: String,
    pub description: String,
    pub nft_rules: Vec<String>,
    pub nfqws_params: Vec<String>,
    pub meta: StrategyMeta,
    pub depends: Vec<String>,
}

impl Strategy {
    pub fn is_valid(&self) -> bool {
        !self.nft_rules.is_empty() && !self.nfqws_params.is_empty()
    }
    
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        
        if self.nft_rules.is_empty() {
            errors.push("Strategy must have at least one nft rule".to_string());
        }
        
        if self.nfqws_params.is_empty() {
            errors.push("Strategy must have at least one nfqws parameter".to_string());
        }
        
        errors
    }
}

#[derive(Debug, Clone)]
pub struct StrategyManager {
    base_dir: String,
    strategies_dir: String,
}

impl StrategyManager {
    pub fn new(base_dir: &str) -> Self {
        let strategies_dir = format!("{}/data/strategies", base_dir);

        Self {
            base_dir: base_dir.to_string(),
            strategies_dir,
        }
    }

    pub fn get_strategies_dir(&self) -> &str {  
        &self.strategies_dir    
    }

    pub fn get_strategy(&self, name: &str) -> Option<Strategy> {
        let strategy_path = format!("{}/{}.json", self.strategies_dir, name);
        
        if !Path::new(&strategy_path).exists() {
            return None;
        }

        let content = fs::read_to_string(&strategy_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn list_strategies(&self) -> Vec<String> {
        if !Path::new(&self.strategies_dir).exists() {
            return vec![];
        }

        let mut strategies = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.strategies_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "json" {
                            if let Some(name) = entry.path().file_stem() {
                                strategies.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        strategies
    }
} 