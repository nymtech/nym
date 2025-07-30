use clap::Parser;
use std::fs;
use std::path::Path;
use std::process::Command;

struct CargoInfo {
    path: String,
    version: String,
    directory: String,
    workspace: bool,
}

#[derive(Parser)]
#[command(name = "cargo-version-scanner")]
#[command(about = "Scan Cargo.toml files in a Rust monorepo and analyze versions")]
struct Args {
    /// Show verbose list of all crates, paths & versions
    #[arg(short, long)]
    verbose: bool,

    /// Only show crates with UNSET versions
    #[arg(short, long)]
    unset_only: bool,

    /// Sort by version instead of path (alphabetical)
    #[arg(long)]
    sort_by_version: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Get all Cargo.toml files using ripgrep
    let output = Command::new("rg")
        .args(&["--files", "--glob", "**/Cargo.toml"])
        .current_dir("..") // Running from /nym/cargo-version-scanner/ so have to go on dir up
        .output()?;

    if !output.status.success() {
        eprintln!(
            "Failed to run ripgrep: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let files_output = String::from_utf8(output.stdout)?;
    let mut cargo_infos = Vec::new();

    println!("Found {} files", files_output.lines().count());
    let mut cargo_files: Vec<&str> = files_output.lines().collect();
    cargo_files.sort();

    for file_path in cargo_files {
        if file_path == "Cargo.toml" || file_path.starts_with("cargo-version-scanner/") {
            continue;
        }

        let dir_name = Path::new(file_path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let (version, workspace) = match get_version_from_cargo_toml(file_path) {
            Ok((Some(v), is_workspace)) => (v, is_workspace),
            Ok((None, is_workspace)) => ("UNSET".to_string(), is_workspace),
            Err(_) => ("ERROR".to_string(), false),
        };

        cargo_infos.push(CargoInfo {
            path: file_path.to_string(),
            version,
            directory: dir_name.to_string(),
            workspace,
        });
    }

    if args.sort_by_version {
        cargo_infos.sort_by(|a, b| a.version.cmp(&b.version));
    } else {
        cargo_infos.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let mut version_counts = std::collections::HashMap::new();
    let mut unset_infos = Vec::new();
    let mut workspace_count = 0;

    for info in &cargo_infos {
        *version_counts.entry(&info.version).or_insert(0) += 1;
        if info.version == "UNSET" {
            unset_infos.push(info);
        }
        if info.workspace {
            workspace_count += 1;
        }
    }

    if args.unset_only {
        println!("\nDirectories with UNSET versions ({}):", unset_infos.len());
        for info in &unset_infos {
            println!("{} {}", info.directory, info.path);
        }
    } else {
        println!("Version distribution:");
        let mut sorted_versions: Vec<_> = version_counts.iter().collect();
        sorted_versions.sort_by_key(|(version, _)| *version);

        for (version, count) in sorted_versions {
            println!("{}: {}", version, count);
        }

        if !unset_infos.is_empty() {
            println!("\nDirectories with UNSET versions ({}):", unset_infos.len());
            for info in &unset_infos {
                println!("{} {}", info.directory, info.path);
            }
        }

        println!("\nTotal crates: {}", cargo_infos.len());
        println!("Workspace inherited: {}", workspace_count);
    }

    if args.verbose {
        let sort_desc = if args.sort_by_version {
            "version"
        } else {
            "path"
        };
        println!("\nAll crates (sorted by {}):", sort_desc);
        for info in &cargo_infos {
            let workspace_indicator = if info.workspace {
                "workspace"
            } else {
                "explicit"
            };
            println!(
                "{:<20} {:<15} {:<10} {}",
                info.directory, info.version, workspace_indicator, info.path
            );
        }
    }

    Ok(())
}

fn get_version_from_cargo_toml(
    file_path: &str,
) -> Result<(Option<String>, bool), Box<dyn std::error::Error>> {
    let full_path = format!("../{}", file_path); // Also have to go one dir up for parsing
    let content = fs::read_to_string(&full_path)?;
    let toml_value: toml::Value = toml::from_str(&content)?;

    // Look for version in [package] section
    if let Some(package) = toml_value.get("package") {
        if let Some(version) = package.get("version") {
            if let Some(version_str) = version.as_str() {
                return Ok((Some(version_str.to_string()), false));
            } else if let Some(version_table) = version.as_table() {
                // Check if it's { workspace = true }
                if let Some(workspace) = version_table.get("workspace") {
                    if workspace.as_bool() == Some(true) {
                        return Ok((Some("workspace".to_string()), true));
                    }
                }
            }
        }
    }

    Ok((None, false))
}
