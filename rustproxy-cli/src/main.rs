use clap::{Parser, Subcommand};
use dirs;
use std::fs;
use std::io;
use toml_edit::{DocumentMut, Item, Table, value};

/// Rust Proxy Configuration Tool
#[derive(Parser, Debug)]
#[command(author, version, about = "CLI tool to set and clear Rust proxy configurations", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Set proxy configuration
    SetProxy {
        /// Proxy name or custom URL
        proxy: String,
    },
    /// Clear proxy configuration
    ClearProxy,
}

/// Predefined proxy services
fn get_predefined_proxies() -> Vec<(&'static str, &'static str)> {
    vec![
        ("rsproxy", "https://rsproxy.cn/crates.io-index/"),
        ("ustc", "https://mirrors.ustc.edu.cn/crates.io-index/"),
        ("tuna", "https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"),
        ("aliyun", "https://mirrors.aliyun.com/crates.io-index/"),
    ]
}

/// Set proxy configuration
fn set_proxy(proxy: &str) -> io::Result<()> {
    // Determine the proxy URL
    let proxy_url = if let Some((_, url)) = get_predefined_proxies()
        .iter()
        .find(|(name, _)| *name == proxy.to_lowercase())
    {
        url.to_string()
    } else if proxy.starts_with("http://") || proxy.starts_with("https://") {
        proxy.to_string()
    } else {
        eprintln!("Error: Unknown proxy name or invalid URL.");
        eprintln!("Available predefined proxy names:");
        for (name, _) in get_predefined_proxies() {
            eprintln!("  - {}", name);
        }
        eprintln!("Or provide a custom URL starting with http:// or https://.");
        std::process::exit(1);
    };

    // Get the config file path
    let config_path = dirs::home_dir()
        .unwrap()
        .join(".cargo")
        .join("config.toml"); // Recommend using config.toml

    // Ensure the .cargo directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read or create the config file
    let mut doc = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        content.parse::<DocumentMut>().unwrap_or_else(|_| DocumentMut::new())
    } else {
        DocumentMut::new()
    };

    // Backup existing config file
    if config_path.exists() {
        let backup_path = config_path.with_extension("backup");
        fs::copy(&config_path, &backup_path)?;
        println!(
            "Existing configuration backed up to {}",
            backup_path.to_string_lossy()
        );
    }

    // Remove existing proxy configuration blocks
    remove_proxy_config(&mut doc);

    // Add new proxy configuration blocks in the desired order
    add_proxy_config(&mut doc, &proxy_url);

    // Write back the updated config file
    fs::write(&config_path, doc.to_string())?;

    println!(
        "Proxy configuration set to {}, config file located at {}",
        proxy_url,
        config_path.to_string_lossy()
    );

    Ok(())
}

/// Clear proxy configuration
fn clear_proxy() -> io::Result<()> {
    // Get the config file path
    let config_path = dirs::home_dir()
        .unwrap()
        .join(".cargo")
        .join("config.toml");

    if !config_path.exists() {
        println!("Configuration file does not exist. Nothing to clear.");
        return Ok(());
    }

    // Backup existing config file
    let backup_path = config_path.with_extension("backup");
    fs::copy(&config_path, &backup_path)?;
    println!(
        "Existing configuration backed up to {}",
        backup_path.to_string_lossy()
    );

    // Read the config file content
    let content = fs::read_to_string(&config_path)?;
    let mut doc = content.parse::<DocumentMut>().unwrap_or_else(|_| DocumentMut::new());

    // Remove proxy-related configuration blocks
    remove_proxy_config(&mut doc);

    // Write back the updated config file
    fs::write(&config_path, doc.to_string())?;

    println!("Proxy configuration has been successfully cleared.");

    Ok(())
}

/// Remove proxy-related configuration blocks
fn remove_proxy_config(doc: &mut DocumentMut) {
    // Remove `replace-with` from [source.crates-io]
    if let Some(source_table) = doc.as_table_mut().get_mut("source") {
        if let Item::Table(source_crates_io) = source_table {
            if let Some(crates_io) = source_crates_io.get_mut("crates-io") {
                if let Item::Table(crates_io_table) = crates_io {
                    crates_io_table.remove("replace-with");
                }
            }

            // Remove [source.rsproxy] and [source.rsproxy-sparse]
            source_crates_io.remove("rsproxy");
            source_crates_io.remove("rsproxy-sparse");
        }
    }

    // Remove [registries.rsproxy]
    if let Some(registries_table) = doc.as_table_mut().get_mut("registries") {
        if let Item::Table(registries) = registries_table {
            registries.remove("rsproxy");
        }
    }

    // Remove [net] git-fetch-with-cli
    if let Some(net_table) = doc.as_table_mut().get_mut("net") {
        if let Item::Table(net) = net_table {
            net.remove("git-fetch-with-cli");
        }
    }
}

/// Add proxy-related configuration blocks in the desired order
fn add_proxy_config(doc: &mut DocumentMut, proxy_url: &str) {
    // Add [source.crates-io]
    if let Some(source_table) = doc.as_table_mut().get_mut("source") {
        if let Item::Table(source_crates_io) = source_table {
            if let Some(crates_io) = source_crates_io.get_mut("crates-io") {
                if let Item::Table(crates_io_table) = crates_io {
                    crates_io_table["replace-with"] = value("rsproxy-sparse");
                }
            } else {
                let mut crates_io_table = Table::new();
                crates_io_table.insert("replace-with", value("rsproxy-sparse"));
                source_crates_io.insert("crates-io", Item::Table(crates_io_table));
            }
        }
    } else {
        // Create [source] table if it doesn't exist
        let mut source_table = Table::new();
        let mut crates_io_table = Table::new();
        crates_io_table.insert("replace-with", value("rsproxy-sparse"));
        source_table.insert("crates-io", Item::Table(crates_io_table));
        doc.insert("source", Item::Table(source_table));
    }

    // Add [source.rsproxy]
    if let Some(source_table) = doc.as_table_mut().get_mut("source") {
        if let Item::Table(source_crates_io) = source_table {
            let mut rsproxy_table = Table::new();
            rsproxy_table.insert("registry", value("https://rsproxy.cn/crates.io-index"));
            source_crates_io.insert("rsproxy", Item::Table(rsproxy_table));
        }
    }

    // Add [source.rsproxy-sparse]
    if let Some(source_table) = doc.as_table_mut().get_mut("source") {
        if let Item::Table(source_crates_io) = source_table {
            let mut rsproxy_sparse_table = Table::new();
            rsproxy_sparse_table.insert("registry", value("sparse+https://rsproxy.cn/index/"));
            source_crates_io.insert("rsproxy-sparse", Item::Table(rsproxy_sparse_table));
        }
    }

    // Add [registries.rsproxy]
    if let Some(registries_table) = doc.as_table_mut().get_mut("registries") {
        if let Item::Table(registries) = registries_table {
            let mut rsproxy_registry = Table::new();
            rsproxy_registry.insert("index", value("https://rsproxy.cn/crates.io-index"));
            registries.insert("rsproxy", Item::Table(rsproxy_registry));
        }
    } else {
        // Create [registries] table if it doesn't exist
        let mut registries_table = Table::new();
        let mut rsproxy_registry = Table::new();
        rsproxy_registry.insert("index", value("https://rsproxy.cn/crates.io-index"));
        registries_table.insert("rsproxy", Item::Table(rsproxy_registry));
        doc.insert("registries", Item::Table(registries_table));
    }

    // Add [net]
    if let Some(net_table) = doc.as_table_mut().get_mut("net") {
        if let Item::Table(net) = net_table {
            net.insert("git-fetch-with-cli", value(true));
        }
    } else {
        // Create [net] table if it doesn't exist
        let mut net_table = Table::new();
        net_table.insert("git-fetch-with-cli", value(true));
        doc.insert("net", Item::Table(net_table));
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::SetProxy { proxy } => set_proxy(&proxy),
        Commands::ClearProxy => clear_proxy(),
    }
}
