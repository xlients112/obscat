use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

use crate::utils::MANIFEST_FILE;

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub package: PackageConfig,
    pub container: ContainerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContainerConfig {
    pub image: String,
    pub source: PathBuf,
    pub destination: String,
    pub command: Vec<String>,
    pub workdir: Option<String>,
    pub ports: Option<String>,
    pub network_mode: Option<String>,
    pub ip_address: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl Manifest {
    pub async fn container_load(package_path: &Path) -> Result<Self, String> {
        let manifest_path = package_path.join(MANIFEST_FILE);

        if !manifest_path.exists() {
            return Err(format!("obscat.toml is not found at {:?}.", package_path));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read manifest file\nerror: {}", e))?;

        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| format!("failed to read manifest file\nerror: {}", e))?;

        Ok(manifest)
    }

    pub fn create_container_id(&self) -> String {
        let sanitized: String = self
            .package
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();

        let short_name: String = sanitized.chars().take(12).collect();
        let trimmed_name = short_name.trim_matches('-');

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let suffix = format!("{:06x}", nanos & 0xFFFFFF);

        if trimmed_name.is_empty() {
            format!("ctr-{}", suffix)
        } else {
            format!("{}-{}", trimmed_name, suffix)
        }
    }
}
