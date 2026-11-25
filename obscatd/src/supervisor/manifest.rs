use std::{fs, path::Path};

use crate::{
    container::manifest::{Manifest as ContainerManifest, ManifestLock as ContainerManifestLock},
    deployment::manifest::{Manifest as DeployManifest, ManifestLock as DeployManifestLock},
    log_error,
    utils::{BUNDLE_PATH, LOCK_FILE},
};

pub fn is_deployment_lock(container_id: &str) -> Result<bool, String> {
    let lock_path = Path::new(BUNDLE_PATH).join(container_id).join(LOCK_FILE);

    if !lock_path.exists() {
        return Err(log_error!("no lockfile found at cache"));
    }

    let content =
        fs::read_to_string(&lock_path).map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

    Ok(content.contains("deployment_id"))
}

impl ContainerManifest {
    pub fn load_container_lock(container_id: &str) -> Result<ContainerManifestLock, String> {
        let lock_path = Path::new(BUNDLE_PATH).join(container_id).join(LOCK_FILE);

        if !lock_path.exists() {
            return Err(log_error!("No lockfile found at cache."));
        }

        let content = fs::read_to_string(&lock_path)
            .map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

        toml::from_str::<ContainerManifestLock>(&content)
            .map_err(|e| log_error!("corrupted lockfile: {}", e))
    }
}

impl DeployManifest {
    pub fn load_deploy_lock(container_id: &str) -> Result<DeployManifestLock, String> {
        let lock_path = Path::new(BUNDLE_PATH).join(container_id).join(LOCK_FILE);

        if !lock_path.exists() {
            return Err(log_error!("No lockfile found at cache."));
        }

        let content = fs::read_to_string(&lock_path)
            .map_err(|e| log_error!("Failed to read lockfile: {}", e))?;

        toml::from_str::<DeployManifestLock>(&content)
            .map_err(|e| log_error!("corrupted lockfile: {}", e))
    }
}
