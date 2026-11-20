use std::path::PathBuf;

use crate::utils::IMAGES_PATH;

#[derive(Debug, Clone)]
pub struct ValidatedService {
    pub name: String,
    pub source_dir: PathBuf,
    pub image_rootfs: PathBuf,
    // pub config: ServiceConfig,
}

pub fn resolve_image_rootfs(image_spec: &str) -> PathBuf {
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

