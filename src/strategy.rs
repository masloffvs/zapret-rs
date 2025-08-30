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
        println!("[DEBUG] StrategyManager::new - base_dir: {}", base_dir);
        println!("[DEBUG] StrategyManager::new - strategies_dir: {}", strategies_dir);
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

    pub fn update_strategies(&self) {
        if !Path::new(&self.strategies_dir).exists() {
            fs::create_dir_all(&self.strategies_dir).unwrap();
            println!("[{}] Created strategies directory: {}", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), self.strategies_dir);
        }

        let repos = self.scan_repositories();
        println!("[{}] Found {} repositories to scan for strategies", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), repos.len());
            
        for (repo_name, bat_files, json_files) in repos {
            println!("[{}] Processing repository: {} (BAT: {}, JSON: {})", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                repo_name, bat_files.len(), json_files.len());
                
            for bat_file in bat_files {
                println!("[{}] Processing BAT file: {}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), bat_file);
                if let Some(strategy) = self.parse_bat_file(&repo_name, &bat_file) {
                    self.save_strategy(&strategy);
                } else {
                    println!("[{}] Warning: Failed to parse BAT file: {}", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), bat_file);
                }
            }
            
            for json_file in json_files {
                println!("[{}] Processing JSON file: {}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), json_file);
                if let Some(strategy) = self.parse_json_file(&repo_name, &json_file) {
                    self.save_strategy(&strategy);
                } else {
                    println!("[{}] Warning: Failed to parse JSON file: {}", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), json_file);
                }
            }
        }
        
        println!("[{}] Strategy update completed", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
    }

    fn scan_repositories(&self) -> Vec<(String, Vec<String>, Vec<String>)> {
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
                        let json_files = self.find_json_files(&repo_path);
                        repos.push((repo_name, bat_files, json_files));
                    }
                }
            }
        }

        repos
    }


    pub fn find_json_files(&self, repo_path: &Path) -> Vec<String> {
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

        let depends = self.extract_dependencies(&bat_content);
        
        Some(Strategy {
            name: bat_name.to_string(),
            repo_name: repo_name.to_string(),
            description: format!("Strategy from {} repository", repo_name),
            nft_rules,
            nfqws_params,
            depends,
            meta: StrategyMeta { 
                version: "1.0".to_string() 
            },
        })
    }

    fn parse_json_file(&self, repo_name: &str, json_name: &str) -> Option<Strategy> {
        let json_path = format!("{}/repos/{}/{}.json", self.base_dir, repo_name, json_name);
        
        if !Path::new(&json_path).exists() {
            return None;
        }

        let json_content = fs::read_to_string(&json_path).ok()?;
        let mut strategy: Strategy = serde_json::from_str(&json_content).ok()?;
        
        // Исправляем repo_name на правильное имя репозитория
        strategy.repo_name = repo_name.to_string();

        Some(strategy)
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

    fn extract_dependencies(&self, bat_content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();
        
        for line in bat_content.lines() {
            let line = line.trim();
            
            // Ищем ссылки на файлы в параметрах nfqws
            if line.contains("--hostlist=") {
                if let Some(start) = line.find("--hostlist=\"") {
                    if let Some(end) = line[start + 12..].find("\"") {
                        let filename = &line[start + 12..start + 12 + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
            
            if line.contains("--ipset=") {
                if let Some(start) = line.find("--ipset=\"") {
                    if let Some(end) = line[start + 9..].find("\"") {
                        let filename = &line[start + 9..start + 9 + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
            
            // Ищем ссылки на bin файлы
            if line.contains("bin/") {
                if let Some(start) = line.find("bin/") {
                    if let Some(end) = line[start..].find("\"") {
                        let filename = &line[start..start + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
        }
        
        dependencies
    }

    fn extract_dependencies_from_params(&self, nfqws_params: &[String]) -> Vec<String> {
        let mut dependencies = Vec::new();
        
        for param in nfqws_params {
            if param.contains("--hostlist=\"") {
                if let Some(start) = param.find("--hostlist=\"") {
                    if let Some(end) = param[start + 12..].find("\"") {
                        let filename = &param[start + 12..start + 12 + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
            
            if param.contains("--ipset=\"") {
                if let Some(start) = param.find("--ipset=\"") {
                    if let Some(end) = param[start + 9..].find("\"") {
                        let filename = &param[start + 9..start + 9 + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
            
            if param.contains("bin/") {
                if let Some(start) = param.find("bin/") {
                    if let Some(end) = param[start..].find("\"") {
                        let filename = &param[start..start + end];
                        if !filename.is_empty() {
                            dependencies.push(filename.to_string());
                        }
                    }
                }
            }
        }
        
        dependencies
    }

    fn save_strategy(&self, strategy: &Strategy) {
        if !strategy.is_valid() {
            let errors = strategy.validation_errors();
            println!("[{}] Warning: Strategy '{}' is invalid and will be skipped: {}", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                strategy.name, errors.join(", "));
            return;
        }
        
        let strategy_path = format!("{}/{}.json", self.strategies_dir, strategy.name);
        let json = serde_json::to_string_pretty(strategy).unwrap();
        fs::write(&strategy_path, json).unwrap();
        
        if !strategy.depends.is_empty() {
            self.copy_dependencies(strategy);
        }
        
        println!("[{}] Strategy '{}' saved successfully", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), strategy.name);
    }

    fn copy_dependencies(&self, strategy: &Strategy) {
        let strategy_dir = format!("{}/{}", self.strategies_dir, strategy.name);
        
        if !Path::new(&strategy_dir).exists() {
            if let Err(e) = fs::create_dir_all(&strategy_dir) {
                eprintln!("Warning: Failed to create strategy directory {}: {}", strategy_dir, e);
                return;
            }
        }
        
        for dependency in &strategy.depends {
            let source_path = format!("{}/repos/{}/{}", self.base_dir, strategy.repo_name, dependency);
            let target_path = format!("{}/{}", strategy_dir, dependency);
            
            if Path::new(&source_path).exists() {
                if let Err(e) = fs::copy(&source_path, &target_path) {
                    eprintln!("Warning: Failed to copy dependency {}: {}", dependency, e);
                } else {
                    println!("[{}] Copied dependency {} for strategy {}", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                        dependency, strategy.name);
                }
            } else {
                println!("[{}] Warning: Dependency {} not found for strategy {}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), 
                    dependency, strategy.name);
            }
        }
    }
} 