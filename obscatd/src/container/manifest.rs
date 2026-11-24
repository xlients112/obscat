use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{log_error, utils::{LOCK_FILE, MANIFEST_FILE}};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub package: PackageConfig,
    pub container: ContainerConfig,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestLock {
    pub container_id: String,
    pub active_containers: Vec<String>,
    pub frozen_manifest: Manifest,
}

impl Manifest {
    pub async fn container_load(package_path: &Path) -> Result<Self, String> {
        let manifest_path = package_path.join(MANIFEST_FILE);

        if !manifest_path.exists() {
            return Err(log_error!("obscat.toml is not found at {:?}.", package_path));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| log_error!("failed to read manifest file\nerror: {}", e))?;

        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| log_error!("failed to read manifest file\nerror: {}", e))?;

        Ok(manifest)
    }

    pub fn write_lock(&self, package_path: &str, container_id: &str) -> Result<(), String> {
        let lock_path = Path::new(package_path).join(LOCK_FILE);

        let mut lock = if lock_path.exists() {
            let content = fs::read_to_string(&lock_path)
                .map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

            toml::from_str::<ManifestLock>(&content).unwrap_or_else(|_| ManifestLock {
                container_id: container_id.to_string(),
                active_containers: Vec::new(),
                frozen_manifest: self.clone(),
            })
        } else {
            ManifestLock {
                container_id: container_id.to_string(),
                active_containers: Vec::new(),
                frozen_manifest: self.clone(),
            }
        };

        if !lock.active_containers.iter().any(|id| id == container_id) {
            lock.active_containers.push(container_id.to_string());
        }

        let tmp_path = Path::new(package_path).join(".obscat.lock.tmp");
        let toml_str = toml::to_string_pretty(&lock)
            .map_err(|e| log_error!("Failed to serialize lockfile: {}", e))?;

        fs::write(&tmp_path, toml_str).map_err(|e| log_error!("error: {}", e))?;
        fs::rename(&tmp_path, &lock_path).map_err(|e| log_error!("Failed to rename {}", e))?;

        Ok(())
    }

    pub fn load_from_lock(package_path: &Path) -> Result<ManifestLock, String> {
        let lock_path = package_path.join(LOCK_FILE);

        if !lock_path.exists() {
            return Err(log_error!("No active deployment lockfile found."));
        }

        let content = fs::read_to_string(&lock_path)
            .map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

        toml::from_str::<ManifestLock>(&content).map_err(|e| log_error!("corrupted lockfile: {}", e))
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
