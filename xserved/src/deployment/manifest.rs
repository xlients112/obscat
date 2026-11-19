use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub package: PackageConfig,
    pub service: HashMap<String, ServiceConfig>,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
}

impl Manifest {
    pub async fn load(package_path: &Path) -> Result<Self, String> {
        let manifest_path = package_path.join("manifest.toml");

        if !manifest_path.exists() {
            return Err(format!(
                "manifest.toml file is not found in {:?}",
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
}

impl ServiceConfig {
    pub fn resolve_source_dir(&self, package_path: &Path, service_name: &str) -> PathBuf {
        match &self.source {
            Some(custom_path) => package_path.join(custom_path),
            None => package_path.join("service").join(service_name),
        }
    }
}
