#[path = "framework.rs"]
mod framework;
#[path = "conf.rs"]
mod conf;
#[path = "repo.rs"]
mod repo;
#[path = "strategy.rs"]
mod strategy;

use framework::ZapretFramework;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = if args.len() > 1 { &args[1] } else { "start" };

    let framework = ZapretFramework::new();

    match command.as_ref() {
        "pull" => {
            framework.pull_repositories();
        }
        "start" => {
            let strategy = if args.len() > 2 { Some(args[2].as_str()) } else { None };
            framework.run(strategy);
        }
        "stop" => {
            framework.stop_nfqws();
        }
        "status" => {
            framework.check_status();
        }
        "restart" => {
            let strategy = if args.len() > 2 { Some(args[2].as_str()) } else { None };
            framework.stop_nfqws();
            std::thread::sleep(std::time::Duration::from_secs(2));
            framework.run(strategy);
        }
        _ => {
            println!("Usage: {} [pull|start [strategy]|stop|status|restart [strategy]]", args[0]);
            std::process::exit(1);
        }
    }
}
