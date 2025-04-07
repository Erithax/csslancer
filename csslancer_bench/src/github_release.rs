use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct Release {
    assets: Vec<ReleaseAsset>,
    tag_name: String,
}

pub fn download_file(url: &str, dest_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading {} to {}", url, dest_path.display());
    let conf = ureq::Agent::config_builder().proxy(None).build();
    let agent = ureq::Agent::new_with_config(conf);
    let response = agent.get(url).call()?;

    if response.status() != 200 {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(dest_path)?;
    io::copy(&mut reader, &mut file)?;

    println!("Downloaded successfully!");
    Ok(())
}

pub enum ReleaseVersion<'a> {
    Latest,
    Tag(&'a str),
}

impl<'a> ReleaseVersion<'a> {
    pub fn to_path(&self) -> String {
        match self {
            ReleaseVersion::Latest => "latest".to_owned(),
            ReleaseVersion::Tag(tag) => {
                "tags/".to_owned() + tag
            }
        }
    }
}

pub fn get_release_asset_url(
    repo_owner: &str,
    repo_name: &str,
    version: ReleaseVersion,
    asset_name_contains: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/{}",
        repo_owner, repo_name, version.to_path()
    );

    println!("Fetching release info from: {}", url);

    let conf = ureq::Agent::config_builder().proxy(None).build();

    let agent = ureq::Agent::new_with_config(conf);

    let response = agent.get(&url)
        .header("Accept", "application/vnd.github+json") // Specify API version
        // .header("Authorization", "Bearer ".to_owned() + std::env::var("GHPAT_RO").unwrap().as_str()) // GHPAT_RO is fine-grained read-only Github PAT
        .header (  "X-GitHub-Api-Version", "2022-11-28")
        .call()
        .unwrap();

    if response.status() != 200 {
        return Err(format!("Failed to get release info: HTTP {}", response.status()).into());
    }

    let body = response.into_body();
    let mut bytes: Vec<u8> = Vec::with_capacity(1000);
    body.into_reader().read_to_end(&mut bytes).unwrap();
    let release: Release = serde_json::from_slice(&bytes).unwrap();

    // println!("Found release: {:?}", release);

    // Find the asset with the specified name
    let asset_url = release
        .assets
        .iter()
        .find(|asset| asset.name.contains(asset_name_contains))
        .map(|asset| asset.browser_download_url.clone())
        .ok_or_else(|| format!("Asset with name containing '{}' not found in release '{}'", asset_name_contains, version.to_path()))?;

    Ok(asset_url)
}
