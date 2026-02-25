use sushi::config::{Config, ConnectionMode};

fn main() {
    let config_path = std::env::var("HOME").unwrap() + "/.sushi.yml";

    match Config::load(&config_path) {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");

            // Display defaults
            if let Some(defaults) = &config.defaults {
                println!("\n🔧 Defaults:");
                println!("  - user: {:?}", defaults.user);
                println!("  - mode: {:?}", defaults.mode);
            }

            match config.resolve() {
                Ok(servers) => {
                    println!("\n✓ {} servers resolved", servers.len());

                    // Group servers by mode
                    let mut direct_count = 0;
                    let mut jump_count = 0;
                    let mut bastion_count = 0;

                    println!("\n📊 Sample servers and their modes:");
                    for server in servers.iter().take(20) {
                        let mode = &server.default_mode;
                        match mode {
                            ConnectionMode::Direct => direct_count += 1,
                            ConnectionMode::Jump => jump_count += 1,
                            ConnectionMode::Bastion => bastion_count += 1,
                        }

                        println!(
                            "  - [{:8}] {}/{}/{}",
                            mode, server.group_name, server.env_name, server.name
                        );
                    }

                    // Count all modes
                    for server in servers.iter().skip(20) {
                        match server.default_mode {
                            ConnectionMode::Direct => direct_count += 1,
                            ConnectionMode::Jump => jump_count += 1,
                            ConnectionMode::Bastion => bastion_count += 1,
                        }
                    }

                    println!("\n📈 Total counts:");
                    println!("  - Direct:  {}", direct_count);
                    println!("  - Jump:    {}", jump_count);
                    println!("  - Bastion: {}", bastion_count);
                }
                Err(e) => {
                    eprintln!("✗ Failed to resolve servers: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to load config: {}", e);
            std::process::exit(1);
        }
    }
}
