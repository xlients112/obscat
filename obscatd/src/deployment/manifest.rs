use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    log_error,
    utils::{LOCK_FILE, MANIFEST_FILE},
};

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
    pub deployment_id: String,
    pub active_containers: Vec<String>,
    pub frozen_manifest: Manifest,
}

static CONTAINER_COUNTER: AtomicU32 = AtomicU32::new(0);

impl Manifest {
    pub async fn load(package_path: &Path) -> Result<Self, String> {
        let manifest_path = package_path.join(MANIFEST_FILE);
        let lock_path = Path::new(package_path).join(LOCK_FILE);

        if lock_path.exists() {
            return Err(
                log_error!("Deployment is already active. Try 'obscat undeploy' first!",).into(),
            );
        }

        if !manifest_path.exists() {
            return Err(log_error!(
                "obscat.toml file is not found in {:?}",
                package_path
            ));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| log_error!("failed to read manifest file\nerror: {}", e))?;

        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| log_error!("failed to read manifest file\nerror: {}", e))?;

        if manifest.service.is_empty() {
            return Err(log_error!("Service applications is empty."));
        }

        Ok(manifest)
    }

    pub fn create_deployment_id(&self) -> String {
        let clean: String = self
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
            .collect::<String>()
            .chars()
            .take(10)
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let suffix = format!("{:04x}", nanos & 0xFFFF);

        if clean.is_empty() {
            format!("dep-{}", suffix)
        } else {
            format!("{}-{}", clean, suffix)
        }
    }

    pub fn create_container_id(&self, service_name: &str) -> String {
        let sanitize = |input: &str, max_len: usize| -> String {
            let s: String = input
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            s.chars()
                .take(max_len)
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        };

        let pkg_part = sanitize(&self.package.name, 8);
        let svc_part = sanitize(service_name, 8);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let count = CONTAINER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let suffix = format!("{:06x}", (nanos ^ count) & 0xFFFFFF);

        match (pkg_part.is_empty(), svc_part.is_empty()) {
            (false, false) => format!("{}-{}-{}", pkg_part, svc_part, suffix),
            (false, true) => format!("{}-{}", pkg_part, suffix),
            (true, false) => format!("{}-{}", svc_part, suffix),
            (true, true) => format!("ctr-{}", suffix),
        }
    }

    pub fn write_lock(
        &self,
        package_path: &str,
        deployment_id: &str,
        container_id: &str,
    ) -> Result<(), String> {
        let lock_path = Path::new(package_path).join(LOCK_FILE);

        let mut lock = if lock_path.exists() {
            let content = fs::read_to_string(&lock_path)
                .map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

            toml::from_str::<ManifestLock>(&content).unwrap_or_else(|_| ManifestLock {
                deployment_id: deployment_id.to_string(),
                active_containers: Vec::new(),
                frozen_manifest: self.clone(),
            })
        } else {
            ManifestLock {
                deployment_id: deployment_id.to_string(),
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

        toml::from_str::<ManifestLock>(&content)
            .map_err(|e| log_error!("corrupted lockfile: {}", e))
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
