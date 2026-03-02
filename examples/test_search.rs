use susshi::config::Config;

fn main() {
    let config_path = std::env::var("HOME").unwrap() + "/.susshi.yml";

    match Config::load(&config_path) {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");

            match config.resolve() {
                Ok(servers) => {
                    println!("\n🔍 Testing search functionality:\n");

                    // Test search by name
                    let name_search = "bdd01";
                    let name_matches: Vec<_> = servers
                        .iter()
                        .filter(|s| s.name.to_lowercase().contains(&name_search.to_lowercase()))
                        .collect();
                    println!(
                        "Search by name '{}': {} results",
                        name_search,
                        name_matches.len()
                    );
                    for s in name_matches.iter().take(3) {
                        println!("  - {}/{}/{}", s.group_name, s.env_name, s.name);
                    }

                    // Test search by host
                    let host_search = "in.phm.education.gouv.fr";
                    let host_matches: Vec<_> = servers
                        .iter()
                        .filter(|s| s.host.to_lowercase().contains(&host_search.to_lowercase()))
                        .collect();
                    println!(
                        "\nSearch by host '{}': {} results",
                        host_search,
                        host_matches.len()
                    );
                    for s in host_matches.iter().take(3) {
                        println!("  - {} -> {}", s.name, s.host);
                    }

                    // Test combined search
                    let combined_search = "colibris";
                    let combined_matches: Vec<_> = servers
                        .iter()
                        .filter(|s| {
                            s.name
                                .to_lowercase()
                                .contains(&combined_search.to_lowercase())
                                || s.host
                                    .to_lowercase()
                                    .contains(&combined_search.to_lowercase())
                        })
                        .collect();
                    println!(
                        "\nSearch by name OR host '{}': {} results",
                        combined_search,
                        combined_matches.len()
                    );
                    for s in combined_matches.iter().take(5) {
                        println!(
                            "  - {}/{}/{} -> {}",
                            s.group_name, s.env_name, s.name, s.host
                        );
                    }
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
