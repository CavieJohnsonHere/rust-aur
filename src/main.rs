// main.rs - Simple AUR Helper (fixed: --github flag as boolean, --meow works alone)
use clap::{Arg, ArgAction, Command};
use reqwest::blocking::get;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::process::Command as Shell;

const AUR_RPC: &str = "https://aur.archlinux.org/rpc/?v=5&";
const GITHUB_AUR_MIRROR: &str = "https://github.com/archlinux/aur.git";

// API response structures
#[derive(Deserialize)]
struct RpcResponse {
    results: Vec<AurPkg>,
}

#[derive(Deserialize, Clone)]
struct AurPkg {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "Popularity")]
    popularity: Option<f32>,
    #[serde(rename = "Maintainer")]
    maintainer: Option<String>,
    #[serde(rename = "Depends")]
    #[serde(default)]
    depends: Vec<String>,
    #[serde(rename = "MakeDepends")]
    #[serde(default)]
    make_depends: Vec<String>,
}

// User prompt helpers
fn prompt_yes(question: &str) -> bool {
    print!("{} [Y/n] ", question);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let resp = input.trim().to_lowercase();
    resp.is_empty() || resp == "y" || resp == "yes"
}

// AUR API functions
fn fetch_search(term: &str) -> Result<Vec<AurPkg>, Box<dyn Error>> {
    let url = format!("{}type=search&arg={}", AUR_RPC, term);
    let resp: RpcResponse = get(&url)?.json()?;

    let mut packages = resp.results;
    packages.sort_by(|a, b| {
        b.popularity
            .unwrap_or(0.0)
            .partial_cmp(&a.popularity.unwrap_or(0.0))
            .unwrap()
    });
    Ok(packages)
}

fn fetch_info(name: &str) -> Result<AurPkg, Box<dyn Error>> {
    let url = format!("{}type=info&arg={}", AUR_RPC, name);
    let resp: RpcResponse = get(&url)?.json()?;

    resp.results
        .into_iter()
        .next()
        .ok_or_else(|| format!("Package '{}' not found", name).into())
}

// --- GitHub mirror helpers ---
fn fetch_github_packages() -> Result<Vec<String>, Box<dyn Error>> {
    let output = Shell::new("git")
        .arg("ls-remote")
        .arg("--heads")
        .arg(GITHUB_AUR_MIRROR)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git ls-remote failed with status: {}",
            output.status
        )
        .into());
    }

    let data = String::from_utf8_lossy(&output.stdout);
    let packages = data
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
        .collect();
    Ok(packages)
}

fn github_package_exists(pkg: &str, list: &[String]) -> bool {
    list.contains(&pkg.to_string())
}

// --- Command implementations ---

fn cmd_search(term: &str, use_github: bool) -> Result<(), Box<dyn Error>> {
    if use_github {
        println!("searching github mirror for '{}'", term);
        let branches = fetch_github_packages()?;
        let mut matches: Vec<&String> = branches.iter().filter(|b| b.contains(term)).collect();
        matches.sort();
        println!("\nFound {} packages (github mirror):", matches.len());
        for pkg in matches {
            println!("\n{}", pkg);
        }
        return Ok(());
    }

    let packages = fetch_search(term)?;
    println!("\nFound {} packages:", packages.len());
    for pkg in packages {
        println!("\n{} {}", pkg.name, pkg.version.as_deref().unwrap_or(""));
        if let Some(desc) = &pkg.description {
            println!("  {}", desc);
        }
        println!("  Popularity: {:.2}", pkg.popularity.unwrap_or(0.0));
    }
    Ok(())
}

fn cmd_install(pkgs: &[String], use_github: bool) -> Result<(), Box<dyn Error>> {
    let github_list = if use_github { Some(fetch_github_packages()?) } else { None };

    for pkg_name in pkgs {
        if use_github {
            if !github_package_exists(pkg_name, github_list.as_ref().unwrap()) {
                eprintln!("package '{}' not found on github mirror, skipping", pkg_name);
                continue;
            }

            println!("\nInstalling from github mirror: {}", pkg_name);
            if !prompt_yes("Proceed?") { println!("Skipping {}", pkg_name); continue; }

            let status = Shell::new("git")
                .arg("clone")
                .arg("--single-branch")
                .arg("--branch")
                .arg(pkg_name)
                .arg(GITHUB_AUR_MIRROR)
                .arg(pkg_name)
                .status()?;

            if !status.success() { eprintln!("git clone failed for {} (mirror).", pkg_name); continue; }

            let remove_deps = prompt_yes("Remove make dependencies after build?");
            let mut args = vec!["-si", "--noconfirm"];
            if remove_deps { args.push("--rmdeps"); }

            let status = Shell::new("makepkg").args(&args).current_dir(pkg_name).status()?;
            let _ = fs::remove_dir_all(pkg_name);

            if status.success() { println!("Successfully installed {}", pkg_name); }
            else { eprintln!("Failed to install {} (build error).", pkg_name); }
        } else {
            let pkg = match fetch_info(pkg_name) {
                Ok(p) => p,
                Err(e) => { eprintln!("failed to fetch info for {}: {}", pkg_name, e); continue; }
            };

            println!("\nInstalling: {} {}", pkg.name, pkg.version.as_deref().unwrap_or(""));
            if !prompt_yes("Proceed?") { println!("Skipping {}", pkg.name); continue; }

            let repo_url = format!("https://aur.archlinux.org/{}.git", pkg.name);
            let status = Shell::new("git").arg("clone").arg(&repo_url).status()?;
            if !status.success() { eprintln!("git clone failed for {} (aur).", pkg.name); continue; }

            let remove_deps = prompt_yes("Remove make dependencies after build?");
            let mut args = vec!["-si", "--noconfirm"];
            if remove_deps { args.push("--rmdeps"); }

            let status = Shell::new("makepkg").args(&args).current_dir(&pkg.name).status()?;
            let _ = fs::remove_dir_all(&pkg.name);

            if status.success() { println!("Successfully installed {}", pkg.name); }
            else { eprintln!("Failed to install {} (build error).", pkg.name); }
        }
    }
    Ok(())
}

fn cmd_update(use_github: bool) -> Result<(), Box<dyn Error>> {
    println!("Updating AUR packages...");
    let output = Shell::new("pacman").arg("-Qm").output()?;
    let aur_pkgs = String::from_utf8_lossy(&output.stdout);
    let pkgs_to_update: Vec<String> = aur_pkgs
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(String::from))
        .collect();
    if pkgs_to_update.is_empty() { println!("No AUR packages installed"); return Ok(()); }
    println!("Found {} packages to update", pkgs_to_update.len());
    cmd_install(&pkgs_to_update, use_github)
}

fn cmd_info(pkg_name: &str, use_github: bool) -> Result<(), Box<dyn Error>> {
    if use_github {
        let branches = fetch_github_packages()?;
        if !github_package_exists(pkg_name, &branches) {
            return Err(format!("package '{}' not found on github mirror", pkg_name).into());
        }
        println!("\nPackage: {} (from github mirror)", pkg_name);
        println!("  source: {}", GITHUB_AUR_MIRROR);
        println!("  note: github mirror provides the PKGBUILD/branch but not AUR metadata (maintainer/popularity).");
        return Ok(());
    }
    let pkg = fetch_info(pkg_name)?;
    println!("\nPackage: {}", pkg.name);
    println!("Version: {}", pkg.version.as_deref().unwrap_or("Unknown"));
    println!("Maintainer: {}", pkg.maintainer.as_deref().unwrap_or("None"));
    println!("Popularity: {:.2}", pkg.popularity.unwrap_or(0.0));
    if !pkg.description.as_ref().map_or(true, |s| s.is_empty()) { println!("\nDescription:\n  {}", pkg.description.unwrap()); }
    if !pkg.depends.is_empty() {
        println!("\nDependencies:"); for dep in &pkg.depends { println!("  - {}", dep); }
    }
    if !pkg.make_depends.is_empty() {
        println!("\nBuild Dependencies:"); for dep in &pkg.make_depends { println!("  - {}", dep); }
    }
    Ok(())
}

fn cmd_clean() -> Result<(), Box<dyn Error>> {
    println!("Cleaning build directories...");
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() { continue; }
        let dir_name = entry.file_name().into_string().unwrap();
        let pkgbuild_path = format!("{}/PKGBUILD", dir_name);
        if fs::metadata(pkgbuild_path).is_ok() {
            fs::remove_dir_all(&dir_name)?;
            println!("Removed: {}", dir_name);
        }
    }
    Ok(())
}

fn cmd_uninstall(pkgs: &[String]) -> Result<(), Box<dyn Error>> {
    for pkg in pkgs {
        if !prompt_yes(&format!("Really uninstall {}?", pkg)) { println!("Skipping {}", pkg); continue; }
        let status = Shell::new("sudo").arg("pacman").arg("-Rns").arg(pkg).status()?;
        if status.success() { println!("Successfully removed {}", pkg); } 
        else { eprintln!("Failed to remove {}", pkg); }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("raur")
        .version("1.2")
        .about("Simple AUR Helper")
        .arg(
            Arg::new("github")
                .long("github")
                .help("Use GitHub mirror instead of AUR RPC (global flag)")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("meow")
                .long("meow")
                .help("meow (necessary feature)")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        // don't require a subcommand at parse time so flags like --meow can be used alone
        .subcommand_required(false)
        .subcommand(Command::new("search").about("Search AUR packages").arg(Arg::new("query").required(true)))
        .subcommand(Command::new("install").about("Install AUR packages").arg(Arg::new("packages").required(true).num_args(1..)))
        .subcommand(Command::new("update").about("Update installed AUR packages"))
        .subcommand(Command::new("info").about("Show package information").arg(Arg::new("package").required(true)))
        .subcommand(Command::new("clean").about("Clean build directories"))
        .subcommand(Command::new("uninstall").about("Uninstall AUR packages").arg(Arg::new("packages").required(true).num_args(1..)))
        .get_matches();

    // allow `raur --meow` to operate on its own
    if matches.get_flag("meow") {
        println!("meow");
        return Ok(());
    }

    // if we reached here and there's no subcommand, show helpful error like clap used to
    let use_github = matches.get_flag("github");
    if matches.subcommand().is_none() {
        eprintln!("error: 'raur' requires a subcommand but one was not provided");
        eprintln!("\nFor more information, try '--help'.");
        std::process::exit(1);
    }

    match matches.subcommand() {
        Some(("search", sub_m)) => cmd_search(sub_m.get_one::<String>("query").unwrap(), use_github)?,
        Some(("install", sub_m)) => {
            let packages: Vec<String> = sub_m.get_many::<String>("packages").unwrap().cloned().collect();
            cmd_install(&packages, use_github)?;
        }
        Some(("update", _)) => cmd_update(use_github)?,
        Some(("info", sub_m)) => cmd_info(sub_m.get_one::<String>("package").unwrap(), use_github)?,
        Some(("clean", _)) => cmd_clean()?,
        Some(("uninstall", sub_m)) => {
            let packages: Vec<String> = sub_m.get_many::<String>("packages").unwrap().cloned().collect();
            cmd_uninstall(&packages)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

