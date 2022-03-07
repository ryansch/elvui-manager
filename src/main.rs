use log::{debug, error, log_enabled, info, Level};
use clap::Parser;
use regex::Regex;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use version_compare::{Cmp, Version};
use scraper::{Html, Selector};

/// Installs / Updates ElvUI
#[derive(Parser, Debug)]
struct Cli {
    /// By default, info logging is enabled.
    /// Passing `-v` one time also prints debug, and -vv trace.
    #[clap(long, short = 'v', parse(from_occurrences))]
    verbose: i8,

    /// The path to the WoW addons directory
    #[clap(parse(from_os_str), default_value = "/Applications/World of Warcraft/_retail_/Interface/Addons" )]
    addons_path: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut builder = env_logger::Builder::from_default_env();
    builder
        .filter(None, verbose_to_log_level(args.verbose).unwrap().to_level_filter())
        .init();

    debug!("args: {:?}", &args);

    let mut install_needed = true;

    // Check installed version
    let result = installed_version(&args.addons_path);
    if result.is_ok() {
        let installed_version = result.unwrap();
        info!("Found installed version: {}", installed_version);

        // Check latest available
        let latest_version = latest_version()?;
        info!("Found latest available version: {}", latest_version);

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
        info!("Installing ElvUI");
        install()?;
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

fn installed_version(addons_path: &PathBuf) -> Result<String> {
    let mut path = PathBuf::from(addons_path);
    path.push("ElvUI/ElvUI_Mainline.toc");

    debug!("Using path: {:?}", &path);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read file `{}`", path.display()))?;

    let re = Regex::new(r"Version: (?P<version>[|\d\.]+)").unwrap();
    let caps = re.captures(&content).unwrap();

    Ok(caps[1].to_string())
}

fn latest_version() -> Result<String> {
    let resp = reqwest::blocking::get("https://www.tukui.org/download.php?ui=elvui")?
        .text()?;

    let document = Html::parse_document(&resp);
    let version_selector = Selector::parse("div#version").unwrap();
    let bold_selector = Selector::parse("b.Premium").unwrap();

    let div = document.select(&version_selector).next()
        .with_context(|| format!("Unable to find {:#?}!", version_selector))?;
    debug!("div = {}", div.html());

    let bold = div.select(&bold_selector).next()
        .with_context(|| format!("Unable to find {:#?}!", bold_selector))?;
    debug!("bold = {}", bold.inner_html());

    let version = bold.inner_html();
    Ok(version)
}

fn install() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_latest_version() {
        let result = latest_version();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "12.66");
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
