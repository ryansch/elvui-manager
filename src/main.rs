use log::{debug, info, Level};
use clap::Parser;
use regex::Regex;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use version_compare::{Cmp, Version};
use tempfile::Builder;
use std::fs::File;
use serde::{Serialize, Deserialize};

/// Installs / Updates ElvUI
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// By default, info logging is enabled.
    /// Passing `-v` one time also prints debug, and `-vv` trace.
    #[clap(long, short = 'v', parse(from_occurrences))]
    verbose: i8,

    /// The path to the WoW addons directory
    #[clap(parse(from_os_str), default_value = "/Applications/World of Warcraft/_retail_/Interface/Addons" )]
    addons_path: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct ElvuiMetadata {
    slug: String,
    name: String,
    url: String,
    version: String,
    changelog_url: String,
    ticket_url: String,
    git_url: String,
    last_update: String,
    directories: Vec<String>,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut builder = env_logger::Builder::from_default_env();
    builder
        .filter(None, verbose_to_log_level(args.verbose).unwrap().to_level_filter())
        .filter_module("html5ever", log::LevelFilter::Info)
        .filter_module("selectors", log::LevelFilter::Info)
        .init();

    debug!("args: {:?}", &args);

    let mut install_needed = true;

    // Check latest available
    let metadata = fetch_metadata()?;
    let latest_version = &metadata.version;
    info!("Found latest available version: {} (updated on {})", latest_version, metadata.last_update);

    // Check installed version
    let result = fetch_installed_version(&args.addons_path);
    if result.is_ok() {
        let installed_version = result.unwrap();
        info!("Found installed version: {}", installed_version);

        let installed = Version::from(&installed_version).unwrap();
        let latest = Version::from(&latest_version).unwrap();

        debug!("Comparing {} to {}", installed, latest);
        install_needed = match installed.compare(latest) {
            Cmp::Lt => true,
            Cmp::Eq => false,
            Cmp::Gt => false,
            _ => unreachable!(),
        };
        debug!("After compare, install_needed = {}", install_needed);
    }

    if install_needed == true {
        info!("Installing ElvUI {}", latest_version);
        install(&args.addons_path, metadata)?;
    }

    Ok(())
}

fn verbose_to_log_level(verbose: i8) -> Result<Level> {
    match verbose {
        0 => Ok(log::Level::Info),
        1 => Ok(log::Level::Debug),
        2 => Ok(log::Level::Trace),
        _ => bail!("Unexpected value {} for verbosity!", verbose)
    }
}

fn fetch_installed_version(addons_path: &PathBuf) -> Result<String> {
    let path = addons_path.join("ElvUI/ElvUI_Mainline.toc");

    debug!("Using path: {:?}", &path);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read file `{}`", path.display()))?;

    let re = Regex::new(r"Version: (?P<version>[|\d\.]+)").unwrap();
    let caps = re.captures(&content).unwrap();

    Ok(caps[1].to_string())
}

fn fetch_metadata() -> Result<ElvuiMetadata> {
    let resp: ElvuiMetadata = reqwest::blocking::get("https://api.tukui.org/v1/addon/elvui")?
        .json()?;
    debug!("json = {:#?}", resp);

    Ok(resp)
}

fn install(addons_path: &PathBuf, metadata: ElvuiMetadata) -> Result<()> {
    if !addons_path.is_dir() {
        bail!("Unable to install! Addons path does not exist!");
    }

    // create temp dir
    let tempdir = Builder::new()
        .prefix("elvui-manager")
        .tempdir()?;
    debug!("tempdir: {:#?}", tempdir);

    // download archive
    let mut response =
        reqwest::blocking::get(metadata.url)?;
    let filename = tempdir.path().join("elvui.zip");
    debug!("filename: {:#?}", &filename);

    let mut file = File::create(&filename)?;
    response.copy_to(&mut file)?;
    debug!("copied response");

    // unzip archive
    let extracted_path = tempdir.path().join("elvui");
    let file = File::open(&filename)?;
    let mut archive = zip::ZipArchive::new(&file).unwrap();
    archive.extract(&extracted_path)?;
    debug!("extracted archive");

    for target in metadata.directories {
        let target_path = addons_path.join(&target);

        // Remove destination path if exists
        if target_path.is_dir() {
            std::fs::remove_dir_all(&target_path)?;
        }

        // Move target from archive to addons dir
        std::fs::rename(
            extracted_path.join(&target),
            &target_path
        )?;
    }

    // Use to keep tempdir for debugging
    // tempdir.into_path();
    tempdir.close()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_metadata() {
        let result = fetch_metadata();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().version, "12.66");
    }

    #[test]
    #[ignore]
    fn latest_version_404() {
    }

    #[test]
    #[ignore]
    fn latest_version_no_version_div() {
    }

    #[test]
    #[ignore]
    fn latest_version_no_bold_div() {
    }

    #[test]
    #[ignore]
    fn latest_version_bad_version() {
    }
}
