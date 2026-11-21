use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::utils::{LOCK_FILE, MANIFEST_FILE};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Manifest {
    pub package: PackageConfig,
    pub service: HashMap<String, ServiceConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ServiceConfig {
    pub image: String,
    pub source: Option<String>,
    pub destination: String,
    pub command: Vec<String>,
    pub workdir: Option<String>,
    pub ports: Option<String>,
    pub network_mode: Option<String>,
    pub ip_address: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestLock {
    pub active_containers: Vec<String>,
    pub frozen_manifest: Manifest,
}

impl Manifest {
    pub async fn load(package_path: &Path) -> Result<Self, String> {
        let manifest_path = package_path.join(MANIFEST_FILE);

        if !manifest_path.exists() {
            return Err(format!(
                "obscat.toml file is not found in {:?}",
                package_path
            ));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read manifest file\nerror: {}", e))?;

        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| format!("failed to read manifest file\nerror: {}", e))?;

        if manifest.service.is_empty() {
            return Err(format!("Service applications is empty."));
        }

        Ok(manifest)
    }

    pub fn write_lock(&self, package_path: &str, container_id: &str) -> Result<(), String> {
        let lock_path = Path::new(package_path).join(LOCK_FILE);

        let mut lock = if lock_path.exists() {
            let content = fs::read_to_string(&lock_path)
            .map_err(|e| format!("Failed to read lockfile: {}", e))?;

        toml::from_str::<ManifestLock>(&content).unwrap_or_else(|_| ManifestLock {
            active_containers: Vec::new(),
            frozen_manifest: self.clone(),
        })
        } else {
            ManifestLock {
                active_containers: Vec::new(),
                frozen_manifest: self.clone(),
            }
        };

        if !lock.active_containers.iter().any(|id| id == container_id) {
            lock.active_containers.push(container_id.to_string());
        }

        let tmp_path = Path::new(package_path).join(".obscat.lock.tmp");
        let toml_str = toml::to_string_pretty(&lock)
        .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;

    fs::write(&tmp_path, toml_str).map_err(|e| format!("error: {}", e))?;
    fs::rename(&tmp_path, &lock_path).map_err(|e| format!("Failed to rename {}", e))?;

    Ok(())

    }

    pub fn load_from_lock(package_path: &Path) -> Result<ManifestLock, String> {
        let lock_path = package_path.join(LOCK_FILE);

        if !lock_path.exists() {
            return Err(format!("No active deployment lockfile found."))
        }

        let content = fs::read_to_string(&lock_path)
        .map_err(|e| format!("Failed to read lockfile: {}", e))?;

    toml::from_str::<ManifestLock>(&content)
    .map_err(|e| format!("corrupted lockfile: {}", e))
    }
}

impl ServiceConfig {
    pub fn resolve_source_dir(&self, package_path: &Path, service_name: &str) -> PathBuf {
        match &self.source {
            Some(custom_path) => package_path.join(custom_path),
            None => package_path.join("service").join(service_name),
        }
    }
}
