use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::{fs, process::Command};

use crate::{
    container::{manifest::Manifest, validate::resolve_image_rootfs}, internal_proto::NetworkConfig, runtime::internal::{container_start, runtime_create, runtime_delete, runtime_kill}, utils::{BUNDLE_PATH, LOCK_FILE, YOUKI_ROOT},
};

pub async fn container_create(
    package_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let package_dir = Path::new(package_path);
    let manifest = Manifest::container_load(package_dir)
        .await
        .map_err(|e| format!("Manifest error: {}", e))?;
    let container_id = manifest.create_container_id();
    manifest.write_lock(package_path, &container_id)?;

    let image_rootfs = resolve_image_rootfs(&manifest.container.image);
    if !fs::try_exists(&image_rootfs).await.unwrap_or(false) {
        println!(
            "Service '{}': image '{}' not found. run it frist 'obscat image pull {}'",
            container_id, manifest.container.image, manifest.container.image
        );
    }

    println!("Preparing container network...");
    let service_net_config = NetworkConfig {
        mode: manifest
            .container
            .network_mode
            .clone()
            .unwrap_or_else(|| "bridge".to_string()),
        ip_addres: manifest.container.ip_address.clone().unwrap_or_default(),
        port: manifest.container.ports.clone().unwrap_or_default(),
    };

    runtime_create(
        &container_id,
        &manifest.container.image,
        &service_net_config,
    )
    .await?;

    let rootfs_dir = PathBuf::from(BUNDLE_PATH)
        .join(&container_id)
        .join("rootfs");

    let clean_dest = manifest.container.destination.trim_start_matches('/');
    let target_app_dir = rootfs_dir.join(clean_dest);
    fs::create_dir_all(&target_app_dir).await?;

    println!(
        "Copying application {:?} to {:?}...",
        manifest.container.source, target_app_dir
    );
    let resolved_source = if manifest.container.source.is_relative() {
        package_dir.join(&manifest.container.source)
    } else {
        manifest.container.source.clone()
    };
    copy_dir_recursive(&resolved_source, &target_app_dir).await?;

    println!("Turn on contianer '{}'...", &container_id);
    container_start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    if !manifest.container.command.is_empty() {
        println!("Running command application.");

        let mut exec_cmd = Command::new("youki");
        exec_cmd.arg("--root").arg(YOUKI_ROOT).arg("exec");

        if let Some(ref workdir) = manifest.container.workdir {
            exec_cmd.arg("--cwd").arg(workdir);
        }

        for (k, v) in &manifest.container.env {
            exec_cmd.arg("--env").arg(format!("{}={}", k, v));
        }

        exec_cmd
            .arg(container_id)
            .args(&manifest.container.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        exec_cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command application: {}", e))?;
    }
    Ok(())
}

pub async fn container_delete(
    package_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pkg_dir = Path::new(package_path);
    let lock = Manifest::load_from_lock(pkg_dir).map_err(|e| format!("Manifest error: {}", e))?;
    let container_id = &lock.container_id;

    println!("Killing container '{}'...", container_id);
    let signal = &format!("SIGKILL").to_string();
    if let Err(e) = runtime_kill(container_id, signal).await {
        println!("Failed to kill container: {}", e);
    } else {
        println!("Container {} has been killed.", container_id);
    }

    println!("Deleting container '{}'...", container_id);
    if let Err(e) = runtime_delete(container_id).await {
        println!("failed to deleting container: {}", e);
    } else {
        println!("sucess to deleting container: {}", container_id);
    }

    let lock_path = pkg_dir.join(LOCK_FILE);
    if lock_path.exists() {
        std::fs::remove_file(&lock_path)
            .map_err(|e| format!("Failed to remove lockfile: {}", e))?;
    }

    println!(
        "Container {} in '{}' have been successfully deleted.",
        container_id,
        package_path
    );

    Ok(())
}

fn copy_dir_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
            + Send
            + 'a,
    >,
> {
    Box::pin(async move {
        let mut entries = fs::read_dir(src).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                fs::create_dir_all(&dst_path).await?;
                copy_dir_recursive(&src_path, &dst_path).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    })
}
