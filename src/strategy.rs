use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone)]
pub struct StrategyManager {
    base_dir: String,
    strategies_dir: String,
}

impl StrategyManager {
    pub fn new(base_dir: &str) -> Self {
        let strategies_dir = format!("{}/strategies", base_dir);
        Self {
            base_dir: base_dir.to_string(),
            strategies_dir,
        }
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

    pub fn update_strategies(&self) {
        if !Path::new(&self.strategies_dir).exists() {
            fs::create_dir_all(&self.strategies_dir).unwrap();
        }

        let repos = self.scan_repositories();
        for (repo_name, bat_files) in repos {
            for bat_file in bat_files {
                if let Some(strategy) = self.parse_bat_file(&repo_name, &bat_file) {
                    self.save_strategy(&strategy);
                }
            }
        }
    }

    fn scan_repositories(&self) -> Vec<(String, Vec<String>)> {
        let repos_dir = format!("{}/repos", self.base_dir);
        let mut repos = Vec::new();

        if !Path::new(&repos_dir).exists() {
            return repos;
        }

        if let Ok(entries) = fs::read_dir(&repos_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let repo_name = entry.file_name().to_string_lossy().to_string();
                    let repo_path = entry.path();
                    
                    if repo_path.is_dir() {
                        let bat_files = self.find_bat_files(&repo_path);
                        repos.push((repo_name, bat_files));
                    }
                }
            }
        }

        repos
    }


    fn find_json_files(&self, repo_path: &Path) -> Vec<String> {
        let mut json_files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(repo_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "json" {
                            if let Some(name) = entry.path().file_stem() {
                                json_files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        json_files
    }

    fn find_bat_files(&self, repo_path: &Path) -> Vec<String> {
        let mut bat_files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(repo_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "bat" {
                            if let Some(name) = entry.path().file_stem() {
                                bat_files.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        bat_files
    }

    fn parse_bat_file(&self, repo_name: &str, bat_name: &str) -> Option<Strategy> {
        let bat_path = format!("{}/repos/{}/{}.bat", self.base_dir, repo_name, bat_name);
        
        if !Path::new(&bat_path).exists() {
            return None;
        }

        let bat_content = fs::read_to_string(&bat_path).ok()?;
        let (nft_rules, nfqws_params) = self.parse_bat_content(&bat_content);

        Some(Strategy {
            name: bat_name.to_string(),
            repo_name: repo_name.to_string(),
            description: format!("Strategy from {} repository", repo_name),
            nft_rules,
            nfqws_params,
            meta: StrategyMeta { 
                version: "1.0".to_string() 
            },
        })
    }

    fn parse_bat_content(&self, bat_content: &str) -> (Vec<String>, Vec<String>) {
        let mut nft_rules = Vec::new();
        let mut nfqws_params = Vec::new();
        let mut queue_num = 0;
        let bin_path = "bin/";

        let mut full_command = String::new();
        let mut found_start = false;
        
        for line in bat_content.lines() {
            let line = line.trim();
            
            if line.starts_with("::") || line.is_empty() || line.starts_with("@echo") || line.starts_with("chcp") || 
               line.starts_with("cd") || line.starts_with("call") || line.starts_with("set") {
                continue;
            }

            if line.starts_with("start") {
                found_start = true;
            }

            if found_start {
                if line.ends_with("^") {
                    full_command.push_str(&line[..line.len()-1]);
                    full_command.push(' ');
                } else {
                    full_command.push_str(line);
                    break;
                }
            }
        }

        full_command = full_command.replace("%BIN%", bin_path);
        let parts: Vec<&str> = full_command.split_whitespace().collect();
        
        for (i, part) in parts.iter().enumerate() {
            if part.contains("winws.exe") {
                let mut current_queue_args = Vec::new();
                
                for j in (i+1)..parts.len() {
                    let arg = parts[j];
                    
                    if arg.starts_with("--filter-tcp=") || arg.starts_with("--filter-udp=") {
                        if !current_queue_args.is_empty() {
                            nfqws_params.push(current_queue_args.join(" "));
                            current_queue_args.clear();
                        }
                        
                        let protocol = if arg.starts_with("--filter-tcp=") { "tcp" } else { "udp" };
                        let ports = arg.split('=').nth(1).unwrap_or("");
                        
                        let nft_rule = format!("{} dport {{{}}} counter queue num {} bypass", protocol, ports, queue_num);
                        nft_rules.push(nft_rule);
                        
                        queue_num += 1;
                    } else if arg == "--new" {
                        if !current_queue_args.is_empty() {
                            nfqws_params.push(current_queue_args.join(" "));
                            current_queue_args.clear();
                        }
                    } else if arg.starts_with("--wf-") {
                        continue;
                    } else {
                        let clean_arg = if arg.starts_with("\"") && arg.ends_with("\"") {
                            &arg[1..arg.len()-1]
                        } else {
                            arg
                        };
                        current_queue_args.push(clean_arg.to_string());
                    }
                }
                
                if !current_queue_args.is_empty() {
                    nfqws_params.push(current_queue_args.join(" "));
                }
                
                break;
            }
        }

        (nft_rules, nfqws_params)
    }

    fn save_strategy(&self, strategy: &Strategy) {
        let strategy_path = format!("{}/{}.json", self.strategies_dir, strategy.name);
        let json = serde_json::to_string_pretty(strategy).unwrap();
        fs::write(&strategy_path, json).unwrap();
    }
} 