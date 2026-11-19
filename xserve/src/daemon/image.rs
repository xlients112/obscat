use std::collections::HashSet;
use std::io::BufReader;

use flate2::bufread::GzDecoder;
use serde::{Deserialize, Serialize};
use tar::Archive;
use tokio::{fs, io::AsyncWriteExt};
use tokio_stream::StreamExt;

use crate::cli::args::VendorList;

const IMAGE_STORE_PATH: &str = "/var/lib/xserved/images";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageRecord {
    pub name: String,
    pub tag: String,
    pub size: u64,
    pub layers: Vec<String>,
}

pub async fn image_pull(
    vendor: &VendorList,
    image: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = match vendor {
        VendorList::DockerHub => {
            println!("switching vendor to: DockerHub");
            println!(
                "WARN: for free user dockerhub have rate limit to 10 pull per hours per network IP\nPlease use pulls carefully"
            );

            vendor_dockerhub(image)
                .await
                .map_err(|e| e as Box<dyn std::error::Error>)
        }
    };
    result
}

pub async fn image_list() -> Result<Vec<(String, String, u64)>, Box<dyn std::error::Error>> {
    let index_path = format!("{}/index.json", IMAGE_STORE_PATH);
    if !fs::try_exists(&index_path).await.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let data = fs::read_to_string(&index_path).await?;
    let images: Vec<ImageRecord> = serde_json::from_str(&data).unwrap_or_default();

    let result = images
        .into_iter()
        .map(|img| (img.name, img.tag, img.size))
        .collect();

    Ok(result)
}

pub async fn image_remove(image_spec: &str) -> Result<(), Box<dyn std::error::Error>> {
    let index_path = format!("{}/index.json", IMAGE_STORE_PATH);
    if !fs::try_exists(&index_path).await.unwrap_or(false) {
        return Err("No local image index found".into());
    }

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

    let data = fs::read_to_string(&index_path).await?;
    let images: Vec<ImageRecord> = serde_json::from_str(&data).unwrap_or_default();

    let mut target_digests: Vec<String> = Vec::new();
    let mut remaining_images: Vec<ImageRecord> = Vec::new();
    let mut found = false;

    for img in images {
        if img.name == repo && img.tag == tag {
            found = true;
            target_digests.extend(img.layers);
        } else {
            remaining_images.push(img);
        }
    }

    if !found {
        return Err(format!("Image {}:{} not found in local index", repo, tag).into());
    }

    let sanitized_name = format!("{}_{}", repo.replace('/', "_"), tag);
    let image_dir_path = format!("{}/{}", IMAGE_STORE_PATH, sanitized_name);
    let rootfs_target = format!("{}/rootfs", image_dir_path);
    if let Ok(mounts) = fs::read_to_string("/proc/mounts").await {
        if mounts.contains(&rootfs_target) {
            return Err(format!(
                "Cannot remove image {}:{}: rootfs is currently mounted by an active container",
                repo, tag
            )
            .into());
        }
    }

    if fs::try_exists(&image_dir_path).await.unwrap_or(false) {
        fs::remove_dir_all(&image_dir_path).await?;
        println!("Removed extracted rootfs at {}", image_dir_path);
    }
    

    let new_data = serde_json::to_string_pretty(&remaining_images)?;
    fs::write(&index_path, new_data).await?;

    let mut active_digests = HashSet::new();
    for img in &remaining_images {
        for digest in &img.layers {
            active_digests.insert(digest.clone());
        }
    }

    let blobs_path = format!("{}/blobs", IMAGE_STORE_PATH);
    for digest in target_digests {
        if !active_digests.contains(&digest) {
            let clean_name = digest.replace("sha256:", "");
            let blob_file_path = format!("{}/{}", blobs_path, clean_name);
            if fs::try_exists(&blob_file_path).await.unwrap_or(false) {
                let _ = fs::remove_file(&blob_file_path).await;
                println!("GC: removed unused blob {}", digest);
            }
        }
    }

    println!("Image {}:{} successfully removed", repo, tag);
    Ok(())
}

// vendor dockerhub
async fn vendor_dockerhub(image: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // pengecekan image
    let (repo, tag) = if let Some((r, t)) = image.split_once(':') {
        (r.to_string(), t.to_string())
    } else {
        (image.to_string(), "latest".to_string())
    };
    let scope_image = if repo.contains('/') {
        repo
    } else {
        format!("library/{}", repo)
    };

    let lower_image_dir = format!("{}_{}", scope_image.replace('/', "_"), tag);
    let rootfs_path = format!("{}/{}/rootfs", IMAGE_STORE_PATH, lower_image_dir);
    let blobs_path = format!("{}/blobs", IMAGE_STORE_PATH);
    fs::create_dir_all(&rootfs_path).await?;
    fs::create_dir_all(&blobs_path).await?;
    let index_path = format!("{}/index.json", IMAGE_STORE_PATH);

    if let Ok(content) = fs::read_to_string(&index_path).await {
        let records: Vec<ImageRecord> = serde_json::from_str(&content).unwrap_or_default();
        if let Some(existing) = records
            .iter()
            .find(|r| r.name == scope_image && r.tag == tag)
        {
            let mut all_blobs_exist = true;
            for digest in &existing.layers {
                let clean_name = digest.replace("sha256:", "");
                let blobs_file = format!("{}/{}", blobs_path, clean_name);
                if !fs::try_exists(&blobs_file).await.unwrap_or(false) {
                    all_blobs_exist = false;
                    break;
                }
            }

            if all_blobs_exist {
                println!("Image {}:{} is already exists in cache", scope_image, tag);

                let rootfs_ready = std::fs::read_dir(&rootfs_path)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false);

                if !rootfs_ready {
                    for digest in &existing.layers {
                        let clean_name = digest.replace("sha256:", "");
                        let tarball_clone = format!("{}/{}", blobs_path, clean_name);
                        let rootfs_clone = rootfs_path.clone();

                        tokio::task::spawn_blocking(
                            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                                let tar_gz = std::fs::File::open(&tarball_clone)?;
                                let tar = GzDecoder::new(BufReader::new(tar_gz));
                                let mut archive = Archive::new(tar);
                                archive.set_preserve_permissions(true);
                                archive.set_unpack_xattrs(true);
                                archive.unpack(&rootfs_clone)?;
                                Ok(())
                            },
                        )
                        .await??;
                    }
                }

                return Ok(());
            };
        }
    }

    let client = reqwest::Client::new();

    let (repo, tag) = if let Some((r, t)) = image.split_once(':') {
        (r.to_string(), t.to_string())
    } else {
        (image.to_string(), "latest".to_string())
    };

    let scope_image = if repo.contains('/') {
        repo
    } else {
        format!("library/{}", repo)
    };

    let auth_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        scope_image
    );
    let auth_res = client.get(&auth_url).send().await?;
    if !auth_res.status().is_success() {
        return Err(format!(
            "failed to authentication to docker.io: status {}",
            auth_res.status()
        )
        .into());
    }

    let auth_json: serde_json::Value = auth_res.json().await?;
    let token = auth_json
        .get("token")
        .or_else(|| auth_json.get("access_token"))
        .and_then(|t| t.as_str())
        .ok_or("Cannot find TOKEN in JSON response")?;

    println!(
        "token secured, fetching manifest for {} ({})",
        scope_image, tag
    );
    let manifest_url = format!(
        "https://registry-1.docker.io/v2/{}/manifests/{}",
        scope_image, tag
    );

    let accept_header = "application/vnd.docker.distribution.manifest.list.v2+json, \
                         application/vnd.oci.image.index.v1+json, \
                         application/vnd.docker.distribution.manifest.v2+json, \
                         application/vnd.oci.image.manifest.v1+json";

    let manifest_res = client
        .get(&manifest_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", accept_header)
        .send()
        .await?;

    if !manifest_res.status().is_success() {
        return Err(format!(
            "failed to get manifest image\nstatus: {}",
            manifest_res.status()
        )
        .into());
    }

    let mut manifest_json: serde_json::Value = manifest_res.json().await?;
    let target_arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };

    if let Some(manifests) = manifest_json.get("manifests").and_then(|m| m.as_array()) {
        let target_digest = manifests
            .iter()
            .find(|m| {
                m.pointer("/platform/os").and_then(|v| v.as_str()) == Some("linux")
                    && m.pointer("/platform/architecture").and_then(|v| v.as_str())
                        == Some(target_arch)
            })
            .and_then(|m| m.get("digest"))
            .and_then(|d| d.as_str())
            .ok_or(format!(
                "Manifest untuk arsitektur linux/{} tidak ditemukan",
                target_arch
            ))?;

        let arch_manifest_url = format!(
            "https://registry-1.docker.io/v2/{}/manifests/{}",
            scope_image, target_digest
        );

        let arch_res = client
            .get(&arch_manifest_url)
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Accept",
                "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json",
            )
            .send()
            .await?;

        if !arch_res.status().is_success() {
            return Err(format!(
                "failed to get architecture manifest\nstatus: {}",
                arch_res.status()
            )
            .into());
        }

        manifest_json = arch_res.json().await?;
    }

    let layers = manifest_json
        .get("layers")
        .and_then(|l| l.as_array())
        .ok_or("Gagal membaca daftar layers dari manifest Docker Hub")?;

    println!("found {} layers, downloading layers", layers.len());

    let mut layer_digests = Vec::new();
    let mut total_size: u64 = 0;

    for (index, layer) in layers.iter().enumerate() {
        let digest = layer
            .get("digest")
            .and_then(|d| d.as_str())
            .ok_or("Digest layer corruption or not found")?;

        layer_digests.push(digest.to_string());
        let clean_name = digest.replace("sha256:", "");
        let cached_blob_path = format!("{}/{}", blobs_path, clean_name);

        if !fs::try_exists(&cached_blob_path).await.unwrap_or(false) {
            println!(
                "Downloading layer [{}/{}]: {}",
                index + 1,
                layers.len(),
                digest
            );

            let blob_url = format!(
                "https://registry-1.docker.io/v2/{}/blobs/{}",
                scope_image, digest
            );

            let response = client
                .get(blob_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(
                    format!("failed to download blob url\nStatus: {}", response.status()).into(),
                );
            }

            let mut file = fs::File::create(&cached_blob_path).await?;
            let mut stream = response.bytes_stream();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result?;
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
        } else {
            println!(
                "Using cached layer [{}/{}]: {}",
                index + 1,
                layers.len(),
                digest
            );
        }

        if let Ok(meta) = fs::metadata(&cached_blob_path).await {
            total_size += meta.len();
        }

        let tarball_clone = cached_blob_path.clone();
        let rootfs_clone = rootfs_path.clone();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let tar_gz = std::fs::File::open(&tarball_clone)?;
                let tar = GzDecoder::new(BufReader::new(tar_gz));
                let mut archive = Archive::new(tar);
                archive.set_preserve_permissions(true);
                archive.set_unpack_xattrs(true);
                archive.unpack(&rootfs_clone)?;
                Ok(())
            },
        )
        .await??;
    }
    let index_path = format!("{}/index.json", IMAGE_STORE_PATH);
    let mut records: Vec<ImageRecord> = if fs::try_exists(&index_path).await.unwrap_or(false) {
        let content = fs::read_to_string(&index_path).await.unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };
    records.retain(|r| !(r.name == scope_image && r.tag == tag));
    records.push(ImageRecord {
        name: scope_image,
        tag,
        size: total_size,
        layers: layer_digests,
    });
    fs::write(&index_path, serde_json::to_string_pretty(&records)?).await?;
    println!("pull image on dockerhub has been success");
    Ok(())
}
