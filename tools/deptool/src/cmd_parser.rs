use crate::{Config, GraphFormat};
use clap::{Arg, ArgAction, Command};
use std::{
    fs,
    path::{Path, PathBuf},
};

static APP_ROOT: &str = "apps";
static CRATE_ROOT: &str = "crates";
static EXAMPLE_ROOT: &str = "examples";
static KERNEL_ROOT: &str = "kernel";
static ULIB_ROOT: &str = "ulib";

/// Ex: exe --default=false --format=mermaid --features=f1 f2 f3
pub fn parse_cmd() -> Result<Config, &'static str> {
    let matches = Command::new("Dependency analysis tool for Arceos")
        .version("1.0")
        .author("ctr")
        .about("Generate d2 or mermaid dependency graph for Arceos based on cargo tree")
        .arg(
            Arg::new("no-default")
                .short('d')
                .long("no-default")
                .action(ArgAction::SetFalse),
        )
        .arg(
            Arg::new("features")
                .short('f')
                .long("name")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("format")
                .short('o')
                .long("format")
                .default_value("mermaid"),
        )
        .arg(Arg::new("target").short('t').long("target").required(true))
        .arg(
            Arg::new("save-path")
                .short('s')
                .long("save-path")
                .default_value("out.txt"),
        )
        .get_matches();

    let is_default = matches.get_flag("no-default");
    let features = matches
        .get_many::<String>("features")
        .unwrap_or_default()
        .map(|f| f.to_string())
        .collect();
    let format = match matches.get_one::<String>("format").unwrap().as_str() {
        "d2" => GraphFormat::D2,
        _ => GraphFormat::Mermaid,
    };
    let target = matches.get_one::<String>("target").unwrap().to_string();
    if !is_arceos_crate(&target) {
        return Err("target not exist, should be valid arceos crate, module or app");
    }

    let loc = target_loc(&target)
        .ok_or("target not exist, should be valid arceos crate, module or app")?;
    let output_loc = matches.get_one::<String>("save-path").unwrap().to_string();
    Ok(gen_config(is_default, features, format, loc, output_loc))
}

fn gen_config(
    is_default: bool,
    features: Vec<String>,
    format: GraphFormat,
    loc: String,
    output_loc: String,
) -> Config {
    Config::build(is_default, features, format, loc, output_loc)
}

pub fn check_crate_name(name: &String) -> bool {
    find_crate(name).is_some()
}

pub fn check_module_name(name: &String) -> bool {
    find_kernel_module(name).is_some()
}

fn target_loc(name: &String) -> Option<String> {
    find_crate(name)
        .or_else(|| find_kernel_module(name))
        .or_else(|| find_app(name))
        .or_else(|| find_example(name))
        .or_else(|| find_lib(name))
}

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .unwrap_or(manifest_dir)
        .to_path_buf()
}

fn workspace_path(path: &str) -> PathBuf {
    workspace_root().join(path)
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn find_crate(name: &String) -> Option<String> {
    fs::read_dir(workspace_path(CRATE_ROOT))
        .ok()?
        .flatten()
        .find_map(|entry| {
            if entry.file_name().to_str() == Some(name) {
                Some(path_to_string(entry.path()))
            } else {
                None
            }
        })
}

fn find_kernel_module(name: &String) -> Option<String> {
    fs::read_dir(workspace_path(KERNEL_ROOT))
        .ok()?
        .flatten()
        .find_map(|domain| {
            let module_path = domain.path().join(name);
            if module_path.join("Cargo.toml").exists() {
                Some(path_to_string(module_path))
            } else {
                None
            }
        })
}

pub fn check_app_name(name: &String) -> bool {
    find_app(name).is_some()
}

pub fn check_example_name(name: &String) -> bool {
    find_example(name).is_some()
}

pub fn check_lib_name(name: &String) -> bool {
    find_lib(name).is_some()
}

pub fn is_arceos_crate(name: &String) -> bool {
    check_crate_name(&name)
        || check_module_name(&name)
        || check_app_name(name)
        || check_example_name(name)
        || check_lib_name(name)
}

pub fn build_loc(name: &String) -> String {
    target_loc(name).unwrap_or_else(|| path_to_string(workspace_path(CRATE_ROOT).join(name)))
}

fn find_app(name: &String) -> Option<String> {
    find_direct_workspace_target(APP_ROOT, name)
}

fn find_example(name: &String) -> Option<String> {
    find_direct_workspace_target(EXAMPLE_ROOT, name)
        .or_else(|| find_direct_workspace_target_by_package_name(EXAMPLE_ROOT, name))
}

fn find_lib(name: &String) -> Option<String> {
    find_direct_workspace_target(ULIB_ROOT, name)
}

fn find_direct_workspace_target(root: &str, name: &String) -> Option<String> {
    let target_path = workspace_path(root).join(name);
    if target_path.join("Cargo.toml").exists() {
        Some(path_to_string(target_path))
    } else {
        None
    }
}

fn find_direct_workspace_target_by_package_name(root: &str, name: &String) -> Option<String> {
    fs::read_dir(workspace_path(root))
        .ok()?
        .flatten()
        .find_map(|entry| {
            let target_path = entry.path();
            if !target_path.is_dir() {
                return None;
            }
            let manifest_path = target_path.join("Cargo.toml");
            if manifest_package_name(&manifest_path).as_deref() == Some(name.as_str()) {
                Some(path_to_string(target_path))
            } else {
                None
            }
        })
}

fn manifest_package_name(manifest_path: &Path) -> Option<String> {
    let manifest = fs::read_to_string(manifest_path).ok()?;
    let mut in_package = false;

    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }

        let value = value.split('#').next().unwrap_or(value).trim();
        let Some(value) = value.strip_prefix('"') else {
            continue;
        };
        let Some((name, _)) = value.split_once('"') else {
            continue;
        };
        return Some(name.to_string());
    }

    None
}
