use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::fs;
use tokio::process::Command;

use crate::{
    deployment::{
        manifest::Manifest,
        validate::{ValidatedService, validate_package},
    }, internal_proto::NetworkConfig, runtime::{
        internal::{container_run, runtime_create, runtime_delete}, logging::LogsCollector, utils::{BUNDLE_PATH, YOUKI_ROOT},
    },
};

pub async fn deploy(
    package_path: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pkg_dir = Path::new(package_path);

    log.info(format!("Reading manifest in {:?}", pkg_dir));
    let manifest = Manifest::load(pkg_dir)
        .await
        .map_err(|e| format!("Manifest error: {}", e))?;

    log.info("Validating package and dependention...");
    let validated_services = validate_package(pkg_dir, &manifest)
        .await
        .map_err(|e| format!("Failed to validate: {}", e))?;

    for svc in validated_services {
        let container_id = &svc.name;
        log.info(format!("Deploying service '{}'...", container_id));

        deploy_service(&svc, log).await?;

        log.info(format!(
            "Service '{}' has been sucess to deploy",
            container_id
        ));
    }

    Ok(())
}

pub async fn undeploy(
    package_path: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pkg_dir = Path::new(package_path);
    let manifest = Manifest::load(pkg_dir).await.map_err(|e| format!("Manifest error: {}", e))?;

    for (service_name, _) in &manifest.service {
        log.info(format!("Menghapus service '{}'...", service_name));

        if let Err(e) = runtime_delete(service_name, log).await {
            log.error(format!("failed to deleting service: {}", e));
        } else {
            log.error(format!("sucess to deleting service: {}", service_name));
            
        }

        log.info(format!("All service in deployment '{}' has been sucess to un-deploy", package_path));
    }

    Ok(())
}

async fn deploy_service(
    svc: &ValidatedService,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let container_id = &svc.name;

    let service_net_config = NetworkConfig {
        mode: svc
            .config
            .network_mode
            .clone()
            .unwrap_or_else(|| "bridge".to_string()),
        ip_addres: svc
            .config
            .ip_address
            .clone()
            .unwrap_or_else(|| "0.0.0.0".to_string()),
        port: svc.config.ports.clone().unwrap_or_default(),
    };

    runtime_create(container_id, &svc.config.image, &service_net_config, log).await?;

    let rootfs_dir = PathBuf::from(BUNDLE_PATH).join(container_id).join("rootfs");

    let clean_dest = svc.config.destination.trim_start_matches('/');
    let target_app_dir = rootfs_dir.join(clean_dest);
    fs::create_dir_all(&target_app_dir).await?;

    log.info(format!(
        "Copying application {:?} to {:?}...",
        svc.source_dir, target_app_dir
    ));
    copy_dir_recursive(&svc.source_dir, &target_app_dir).await?;

    log.info(format!("Turn on contianer '{}'...", container_id));
    container_run(container_id, log)
        .await
        .map_err(|e| e.to_string())?;

    if !svc.config.command.is_empty() {
        log.info(format!(
            "Running user application: {:?}",
            svc.config.command
        ));

        let mut exec_cmd = Command::new("youki");
        exec_cmd.arg("--root").arg(YOUKI_ROOT).arg("exec");

        if let Some(ref workdir) = svc.config.workdir {
            exec_cmd.arg("--cwd").arg(workdir);
        }

        for (k, v) in &svc.config.env {
            exec_cmd.arg("--env").arg(format!("{}={}", k, v));
        }

        exec_cmd
            .arg(container_id)
            .args(&svc.config.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        exec_cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command application: {}", e))?;
    }

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
