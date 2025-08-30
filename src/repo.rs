use std::fs;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

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
        let repo_list_path = format!("{}/repo.list", base_dir);
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
        let repo_dir = format!("{}/repos/{}", self.base_dir, repo.name);

        if Path::new(&repo_dir).exists() {
            self.log(&format!("Updating repository: {}", repo.name));
            Command::new("git")
                .arg("pull")
                .current_dir(&repo_dir)
                .output()
                .unwrap();
        } else {
            self.log(&format!("Cloning repository: {}", repo.name));
            fs::create_dir_all(format!("{}/repos", self.base_dir)).unwrap();
            
            Command::new("git")
                .arg("clone")
                .arg(&repo.url)
                .arg(&repo_dir)
                .output()
                .unwrap();
        }

        if let Some(branch) = &repo.branch {
            Command::new("git")
                .arg("checkout")
                .arg(branch)
                .current_dir(&repo_dir)
                .output()
                .unwrap();
        }

        if let Some(commit) = &repo.commit {
            Command::new("git")
                .arg("checkout")
                .arg(commit)
                .current_dir(&repo_dir)
                .output()
                .unwrap();
        }
    }

    pub fn update_all(&self) {
        let repos = self.load_repo_list();
        for repo in repos {
            self.update_repo(&repo);
        }
    }

    fn log(&self, message: &str) {
        println!("[{}] {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), message);
    }
}
