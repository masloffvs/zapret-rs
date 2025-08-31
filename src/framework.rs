use std::process::Command;
use std::fs;
use std::path::Path;
use std::env;
use crate::conf::ConfResult;
use crate::repo::RepoManager;
use crate::strategy::{self, StrategyManager};

#[derive(Debug, Clone)]
pub struct NetInterface {
  pub name: String,
}

impl NetInterface {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub fn get_all() -> Vec<Self> {
      let interfaces: Vec<String> = fs::read_dir("/sys/class/net").unwrap().map(|entry| entry.unwrap().path().to_string_lossy().to_string()).collect();
      interfaces.into_iter().map(|interface| {
        let name = interface.split("/").last().unwrap().to_string();
        NetInterface::new(&name)
      }).collect()
    
    }
}

pub fn get_net_interfaces() -> Vec<NetInterface> {
    let interfaces = NetInterface::get_all();
    interfaces
}

pub fn get_net_interface_by_name(name: &str) -> Option<NetInterface> {
    let interfaces = NetInterface::get_all();
    interfaces.into_iter().find(|interface| interface.name == name)
}

pub struct ZapretFramework {
    conf: ConfResult,
    repo_manager: RepoManager,
    strategy_manager: StrategyManager,
    base_dir: String,
    nfqws_path: String,
    stop_script: String,
}

impl ZapretFramework {
    pub fn new() -> Self {
        let base_dir = env::current_dir().unwrap().to_string_lossy().to_string();
        let nfqws_path = format!("{}/bin/nfqws", base_dir);
        let stop_script = format!("{}/stop_and_clean_nft.sh", base_dir);
        
        let conf = crate::conf::Conf::load(&format!("{}/data/conf.json", base_dir));
        let repo_manager = RepoManager::new(&base_dir);
        let strategy_manager = StrategyManager::new(&base_dir);
        
        Self {
            conf,
            repo_manager,
            strategy_manager,
            base_dir,
            nfqws_path,
            stop_script,
        }
    }

    pub fn log(&self, message: &str) {
        println!("[{}] {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), message);
    }


    pub fn handle_error(&self, message: &str) -> ! {
        self.log(&format!("Error: {}", message));
        std::process::exit(1);
    }

    pub fn check_dependencies(&self) {
        let deps = vec!["git", "nft", "grep", "sed"];
        for dep in deps {
            if Command::new("which").arg(dep).output().unwrap().status.success() {
                self.log(&format!("Utility {} found", dep));
            } else {
                self.handle_error(&format!("Utility {} not installed", dep));
            }
        }
    }

    pub fn setup_repositories(&self) {
        self.log("Setting up repositories...");
        self.repo_manager.update_all();
        self.log("Repository setup completed");
    }

    pub fn setup_nftables(&self, interface: &str, strategy_name: &str) {
        self.log("Setting up nftables...");
        
        let strategy = self.strategy_manager.get_strategy(strategy_name);
        if strategy.is_none() {
            self.handle_error(&format!("Strategy {} not found. Looking in {} directory", strategy_name, self.strategy_manager.get_strategies_dir()));
        }

        let strategy = strategy.unwrap();
        let table_name = "inet zapretunix";
        let chain_name = "output";
        let rule_comment = "Added by zapret script";

        let output = Command::new("sudo")
            .arg("nft")
            .arg("list")
            .arg("tables")
            .output()
            .unwrap();

        if output.status.success() {
            let tables = String::from_utf8_lossy(&output.stdout);
            if tables.contains(table_name) {
                self.log("Removing existing nftables table...");
                Command::new("sudo")
                    .arg("nft")
                    .arg("flush")
                    .arg("chain")
                    .arg(table_name)
                    .arg(chain_name)
                    .output()
                    .unwrap();
                
                Command::new("sudo")
                    .arg("nft")
                    .arg("delete")
                    .arg("chain")
                    .arg(table_name)
                    .arg(chain_name)
                    .output()
                    .unwrap();
                
                Command::new("sudo")
                    .arg("nft")
                    .arg("delete")
                    .arg("table")
                    .arg(table_name)
                    .output()
                    .unwrap();
            }
        }

        Command::new("sudo")
            .arg("nft")
            .arg("add")
            .arg("table")
            .arg(table_name)
            .output()
            .unwrap();

        Command::new("sudo")
            .arg("nft")
            .arg("add")
            .arg("chain")
            .arg(table_name)
            .arg(chain_name)
            .arg("{ type filter hook output priority 0; }")
            .output()
            .unwrap();

        for (queue_num, rule) in strategy.nft_rules.iter().enumerate() {
            let full_rule = format!("oifname \"{}\" {} comment \"{}\"", interface, rule, rule_comment);
            
            let output = Command::new("sudo")
                .arg("nft")
                .arg("add")
                .arg("rule")
                .arg(table_name)
                .arg(chain_name)
                .arg(&full_rule)
                .output()
                .unwrap();

            if !output.status.success() {
                self.log(&format!("Warning: error adding rule for queue {}", queue_num));
            } else {
                self.log(&format!("Added rule for queue {}: {}", queue_num, rule));
            }
        }
    }

    pub fn start_nfqws(&self, interface: &str, strategy_name: &str) {
        self.log("Starting nfqws...");
        
        Command::new("sudo")
            .arg("pkill")
            .arg("-f")
            .arg("nfqws")
            .output()
            .unwrap();

        let strategy = self.strategy_manager.get_strategy(strategy_name);
        if strategy.is_none() {
            self.handle_error(&format!("Strategy {} not found. Looking in {} directory", strategy_name, self.strategy_manager.get_strategies_dir()));
        }

        let strategy = strategy.unwrap();
        let data_strategies_dir = format!("{}/data/strategies/{}", self.base_dir, strategy.repo_name);

        let nfqws_abs_path = Path::new(&self.nfqws_path).canonicalize().unwrap();

        println!("[{}] Jumping to {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), data_strategies_dir);
        env::set_current_dir(&data_strategies_dir).unwrap();

        for (queue_num, params) in strategy.nfqws_params.iter().enumerate() {
            let queue_num_str = format!("--qnum={}", queue_num);
            let mut args = vec!["--daemon", &queue_num_str];
            
            let clean_params = params.replace("\"", "");
            let params_split: Vec<&str> = clean_params.split_whitespace().collect();
            args.extend(params_split);

            let output = Command::new("sudo")
                .arg(&nfqws_abs_path)
                .args(&args)
                .current_dir(&data_strategies_dir)
                .output()
                .unwrap();

            if output.status.success() {
                self.log(&format!("nfqws started for queue {}", queue_num));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
              
                self.log(&format!("Error starting nfqws (queue {}): stderr: {}, stdout: {}, nfqws_path: {}, args: {}", 
                    queue_num, stderr, stdout, self.nfqws_path, args.join(" ")));
            }
        }

        env::set_current_dir(&self.base_dir).unwrap();
    }

    pub fn stop_nfqws(&self) {
        self.log("Stopping nfqws...");
        
        let output = Command::new("pkill")
            .arg("-f")
            .arg("nfqws")
            .output()
            .unwrap();

        if output.status.success() {
            self.log("nfqws stopped");
        } else {
            self.log("nfqws was not running");
        }

        if Path::new(&self.stop_script).exists() {
            let output = Command::new("bash")
                .arg(&self.stop_script)
                .output()
                .unwrap();

            if output.status.success() {
                self.log("nftables cleared");
            }
        }
    }

    pub fn check_status(&self) {
        self.log("Checking status...");
        
        let output = Command::new("pgrep")
            .arg("-f")
            .arg("nfqws")
            .output()
            .unwrap();

        if output.status.success() {
            let pids: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect();
            
            self.log(&format!("nfqws running (PID: {})", pids.join(", ")));
        } else {
            self.log("nfqws not running");
        }

        let output = Command::new("nft")
            .arg("list")
            .arg("ruleset")
            .output()
            .unwrap();

        if output.status.success() {
            let rules = String::from_utf8_lossy(&output.stdout);
            if rules.contains("zapret") {
                self.log("nftables rules configured");
            } else {
                self.log("nftables rules not found");
            }
        }
    }

    pub fn pull_repositories(&self) {
        self.log("Pulling and indexing repositories...");
        self.setup_repositories();
    }

    pub fn run(&self, strategy_override: Option<&str>) {
        self.check_dependencies();
        
        if self.conf.is_fallback {
            self.log("Using default configuration");
        }

        let interface = &self.conf.conf.interface;
        let strategy = strategy_override.unwrap_or(&self.conf.conf.strategy);

        if interface.is_empty() {
            self.handle_error("Interface not specified in configuration");
        }

        if strategy.is_empty() {
            self.handle_error("Strategy not specified in configuration or command line");
        }

        // checl depends 
        let strategy_obj = self.strategy_manager.get_strategy(strategy);
        if strategy_obj.is_none() {
            self.handle_error(&format!("Strategy {} not found. Looking in {} directory", strategy, self.strategy_manager.get_strategies_dir()));
        }

        let strategy_obj = strategy_obj.unwrap();
        let strategies_abs_dir = format!("{}/data/strategies", self.base_dir);
        let strategies_abs_dir = Path::new(&strategies_abs_dir).canonicalize().unwrap();

        for depend in strategy_obj.depends.iter() {
            if !Path::new(&strategies_abs_dir).join(&strategy_obj.repo_name).join(depend).exists() {
                self.handle_error(&format!("Dependency '{}' not found", depend));
            }
        }

        // self.setup_repositories();
        self.setup_nftables(interface, strategy);
        self.start_nfqws(interface, strategy);

        self.log("Zapret system successfully started");
    }
}