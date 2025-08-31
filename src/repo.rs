use std::fs;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub name: String,
    pub url: String,
    pub branch: Option<String>,
    pub commit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RepoManager {
    base_dir: String,
    repo_list_path: String,
}

impl RepoManager {
    pub fn new(base_dir: &str) -> Self {
        let repo_list_path = format!("{}/data/repo.list", base_dir);
        let repo_list_default_path = format!("{}/repo.list.default", base_dir);
        
        let data_dir = format!("{}/data", base_dir);
        if !Path::new(&data_dir).exists() {
            if let Err(e) = fs::create_dir_all(&data_dir) {
                eprintln!("Warning: Failed to create data directory: {}", e);
            } else {
                println!("[{}] Created data directory: {}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), data_dir);
            }
        }
        
        if !Path::new(&repo_list_path).exists() && Path::new(&repo_list_default_path).exists() {
            if let Ok(content) = fs::read_to_string(&repo_list_default_path) {
                if let Err(e) = fs::write(&repo_list_path, content) {
                    eprintln!("Warning: Failed to create repo.list from default: {}", e);
                } else {
                    println!("[{}] Created repo.list from repo.list.default in data/", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }
        
        Self {
            base_dir: base_dir.to_string(),
            repo_list_path,
        }
    }

    pub fn load_repo_list(&self) -> Vec<RepoEntry> {
        if !Path::new(&self.repo_list_path).exists() {
            return vec![];
        }

        let content = fs::read_to_string(&self.repo_list_path).unwrap_or_default();
        let mut repos = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let branch = if parts.len() > 2 { Some(parts[2].to_string()) } else { None };
                let commit = if parts.len() > 3 { Some(parts[3].to_string()) } else { None };

                repos.push(RepoEntry {
                    name,
                    url,
                    branch,
                    commit,
                });
            }
        }

        repos
    }

    pub fn update_repo(&self, repo: &RepoEntry) {
        let repo_dir = format!("{}/data/strategies/{}", self.base_dir, repo.name);
        // is git?
        if  Path::new(&repo_dir).exists() && !Path::new(&repo_dir).join(".git").exists() {
            self.log(&format!("Repository '{}' in '{}' is not a git repository, skipping", repo.name, repo_dir));
            return;
        }


        if Path::new(&repo_dir).exists() {
            self.log(&format!("Updating existing repository: {}", repo.name));
            let output = Command::new("git")
                .arg("pull")
                .current_dir(&repo_dir)
                .output()
                .unwrap();
            
            if output.status.success() {
                self.log(&format!("Repository {} updated successfully", repo.name));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.log(&format!("Warning: git pull failed for {}: {}", repo.name, stderr));
            }
        } else {
            self.log(&format!("Cloning new repository: {} from {}", repo.name, repo.url));
            fs::create_dir_all(format!("{}/data/strategies", self.base_dir)).unwrap();
            
            let output = Command::new("git")
                .arg("clone")
                .arg(&repo.url)
                .arg(&repo_dir)
                .output()
                .unwrap();
                
            if output.status.success() {
                self.log(&format!("Repository {} cloned successfully", repo.name));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.log(&format!("Error cloning repository {}: {}", repo.name, stderr));
                return;
            }
        }

        if let Some(branch) = &repo.branch {
            self.log(&format!("Checking out branch {} for repository {}", branch, repo.name));
            let output = Command::new("git")
                .arg("checkout")
                .arg(branch)
                .current_dir(&repo_dir)
                .output()
                .unwrap();
                
            if output.status.success() {
                self.log(&format!("Branch {} checked out successfully for {}", branch, repo.name));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.log(&format!("Warning: branch checkout failed for {}: {}", repo.name, stderr));
            }
        }

        if let Some(commit) = &repo.commit {
            self.log(&format!("Checking out commit {} for repository {}", commit, repo.name));
            let output = Command::new("git")
                .arg("checkout")
                .arg(commit)
                .current_dir(&repo_dir)
                .output()
                .unwrap();
                
            if output.status.success() {
                self.log(&format!("Commit {} checked out successfully for {}", commit, repo.name));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.log(&format!("Warning: commit checkout failed for {}: {}", repo.name, stderr));
            }
        }
        
        self.log(&format!("Repository {} processing completed", repo.name));
    }

    pub fn update_all(&self) {
        let repos = self.load_repo_list();
        self.log(&format!("Found {} repositories to process", repos.len()));
        for (i, repo) in repos.iter().enumerate() {
            self.log(&format!("Processing repository {}/{}: {}", i + 1, repos.len(), repo.name));
            self.update_repo(repo);
        }
        self.log("All repositories processed");
    }

    fn log(&self, message: &str) {
        println!("[{}] {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), message);
    }
}
