use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use tokio::fs;

use crate::{
    container::{manifest::Manifest, validate::resolve_image_rootfs},
    internal_proto::NetworkConfig,
    log_error, log_info, log_success,
    runtime::{
        container_internal::container_cmd,
        db,
        internal::{container_start, runtime_create, runtime_delete, runtime_kill},
        network,
    },
    utils::{BUNDLE_PATH, LOCK_FILE, YOUKI_ROOT},
};

pub async fn container_create(
    package_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let package_dir = Path::new(package_path);
    let manifest = Manifest::container_load(package_dir)
        .await
        .map_err(|e| log_error!("Manifest error: {}", e))?;
    let container_id = manifest.create_container_id();
    manifest.write_lock(package_path, &container_id)?;

    log_info!("Validating images..");
    let image_rootfs = resolve_image_rootfs(&manifest.container.image);
    if !fs::try_exists(&image_rootfs).await.unwrap_or(false) {
        log_error!(
            "Service '{}': image '{}' not found. run it frist 'obscat image pull {}'",
            container_id,
            manifest.container.image,
            manifest.container.image
        );
    }

    log_info!("Preparing container network...");
    let service_net_config = NetworkConfig {
        mode: manifest
            .container
            .network_mode
            .clone()
            .unwrap_or_else(|| "bridge".to_string()),
        ip_addres: manifest.container.ip_address.clone().unwrap_or_default(),
        port: manifest.container.ports.clone().unwrap_or_default(),
    };

    log_info!("Creating container...");
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

    log_info!(
        "Copying application {:?} to {:?}...",
        manifest.container.source,
        target_app_dir
    );
    let resolved_source = if manifest.container.source.is_relative() {
        package_dir.join(&manifest.container.source)
    } else {
        manifest.container.source.clone()
    };
    copy_dir_recursive(&resolved_source, &target_app_dir).await?;

    log_info!("Turn on contianer '{}'...", &container_id);
    container_start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    if !manifest.container.command.is_empty() {
        log_info!("Running command application.");

        let mut exec_cmd = container_cmd()?;
        exec_cmd.arg("--root").arg(YOUKI_ROOT).arg("exec");

        if let Some(ref workdir) = manifest.container.workdir {
            exec_cmd.arg("--cwd").arg(workdir);
        }

        for (k, v) in &manifest.container.env {
            exec_cmd.arg("--env").arg(format!("{}={}", k, v));
        }

        exec_cmd
            .arg(&container_id)
            .args(&manifest.container.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        exec_cmd
            .spawn()
            .map_err(|e| log_error!("Failed to spawn command application: {}", e))?;
    }

    // copy lockfile to bundles
    let create_lock = Path::new(package_path).join(LOCK_FILE);
    let bundle_lock = Path::new(BUNDLE_PATH).join(&container_id).join(LOCK_FILE);

    if create_lock.exists() {
        fs::copy(create_lock, bundle_lock).await?;
    }

    let final_ip = network::allocated_ip(&container_id, &service_net_config.ip_addres);
    log_info!("Creating database for container {}", &container_id);
    db::container::save(
        &container_id,
        &manifest.package.name,
        &manifest.container.image,
        &final_ip,
        &service_net_config.port,
    )
    .map_err(|e| log_error!("failed to save container info to database, error: {}", e))?;

    log_success!("Container has been sucess to create.");

    Ok(())
}

pub async fn container_delete(
    package_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pkg_dir = Path::new(package_path);

    let lock = Manifest::load_from_lock(pkg_dir).map_err(|e| format!("Manifest error: {}", e))?;

    let container_id = &lock.container_id;

    log_info!("Getting database information");
    db::container::get_network(&container_id)
        .map_err(|e| log_error!("failed to get database info, error: {}", e))?;

    log_info!("Killing container '{}'...", container_id);
    let signal = &format!("SIGKILL").to_string();
    if let Err(e) = runtime_kill(container_id, signal).await {
        log_error!("Failed to kill container: {}", e);
    } else {
        log_info!("Container {} has been killed.", container_id);
    }

    log_info!("Deleting container '{}'...", container_id);
    if let Err(e) = runtime_delete(container_id).await {
        log_error!("failed to deleting container: {}", e);
    } else {
        log_info!("sucess to deleting container: {}", container_id);
    }

    let lock_path = pkg_dir.join(LOCK_FILE);
    if lock_path.exists() {
        std::fs::remove_file(&lock_path)
            .map_err(|e| log_error!("Failed to remove lockfile: {}", e))?;
    }

    log_info!("Cleaning database container");
    db::container::delete(&container_id)
        .map_err(|e| log_error!("failed to deleting container database, error: {}", e))?;

    log_success!(
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
