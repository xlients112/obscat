use std::path::{Path, PathBuf};

use tokio::fs;

use crate::{
    deployment::manifest::{Manifest, ServiceConfig},
    runtime::utils::IMAGES_PATH,
};

#[derive(Debug, Clone)]
pub struct ValidatedService {
    pub name: String,
    pub source_dir: PathBuf,
    pub image_rootfs: PathBuf,
    pub config: ServiceConfig,
}

fn resolve_image_rootfs(image_spec: &str) -> PathBuf {
    let (raw_repo, tag) = if let Some((r, t)) = image_spec.split_once(':') {
        (r.to_string(), t.to_string())
    } else {
        (image_spec.to_string(), "latest".to_string())
    };

    let repo = if raw_repo.contains('/') {
        raw_repo
    } else {
        format!("library/{}", raw_repo)
    };

    let sanitized_name = format!("{}_{}", repo.replace('/', "_"), tag);
    PathBuf::from(format!("{}/{}/rootfs", IMAGES_PATH, sanitized_name))
}

pub async fn validate_package(
    package_dir: &Path,
    manifest: &Manifest,
) -> Result<Vec<ValidatedService>, String> {
    if !package_dir.exists() || !package_dir.is_dir() {
        return Err(format!("Directory {:?} is invalid", package_dir));
    }

    let mut validate_service = Vec::new();

    for (service_name, config) in &manifest.service {
        if !config.destination.starts_with('/') {
            return Err(format!(
                "Service '{}': destination must be '/' in frist",
                service_name
            ));
        }

        if let Some(ref port_map) = config.ports {
            if !port_map.is_empty() && !port_map.contains(':') {
                return Err(format!(
                    "Service '{}': format ports '{}' is invalid (use 'host_port:container_port', example: '8080:80')",
                    service_name, port_map
                ));
            }
        }

        let source_dir = config.resolve_source_dir(package_dir, service_name);
        if !source_dir.is_dir() {
            return Err(format!(
                "Service '{}': folder is not found at {:?}",
                service_name, source_dir
            ));
        }

        let image_rootfs = resolve_image_rootfs(&config.image);
        if !fs::try_exists(&image_rootfs).await.unwrap_or(false) {
            return Err(format!(
                "Service '{}': image '{}' not found. run it frist 'xserve image pull {}'",
                service_name, config.image, config.image
            ));
        }

        validate_service.push(ValidatedService {
            name: service_name.clone(),
            source_dir,
            image_rootfs,
            config: config.clone(),
        });
    }

    Ok(validate_service)
}
