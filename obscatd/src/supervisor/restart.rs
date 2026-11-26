use std::{error::Error, path::Path, process::Stdio};

use tokio::process::Command;

use crate::{
    container::manifest::Manifest as ContainerManifest,
    deployment::manifest::Manifest as DeployManifest,
    internal_proto::NetworkConfig,
    log_error, log_info, log_success,
    runtime::{
        container_internal::container_cmd,
        db,
        internal::{container_start, get_container_pid, resolve_image, runtime_kill},
        network,
    },
    supervisor::manifest::is_deployment_lock,
    utils::{BUNDLE_PATH, YOUKI_ROOT},
};

// restart from check
pub async fn restart(container_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    if is_deployment_lock(container_id)? {
        restart_deployment(container_id).await?;
    } else {
        restart_container(container_id).await?;
    }
    Ok(())
}

// deployment container
pub async fn restart_deployment(container_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    log_info!("Restarting your container with reading the cache manifest lock");
    let manifest = DeployManifest::load_deploy_lock(container_id)?;
    let pkg = &manifest.frozen_manifest.package.name;
    let (_, svc) = manifest
        .frozen_manifest
        .service
        .iter()
        .find(|(svc_name, _)| container_id.starts_with(&format!("{}-{}-", pkg, svc_name)))
        .ok_or_else(|| log_error!("Service '{}' tidak ditemukan di lockfile", container_id))?;

    log_info!("Checking bundles folder..");
    let bundles = Path::new(BUNDLE_PATH).join(container_id);
    if !bundles.exists() {
        return Err(log_error!("Bundles folder is not found.").into());
    }

    // read db
    log_info!("Getting database information...");
    db::container::get_network(container_id)
        .map_err(|e| log_error!("failed to get database info, error: {}", e))?;

    // kill contianer
    log_info!("killing the container...");
    let signal = &format!("SIGKILL").to_string();
    if let Err(e) = runtime_kill(container_id, signal).await {
        log_error!("Failed to kill container: {}", e);
    } else {
        log_info!("Container {} has been killed.", container_id);
    }

    // delete container
    log_info!("Recreating container instance...");
    let mut delete = container_cmd()?;
    delete
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("delete")
        .arg("--force")
        .arg(container_id);

    let output = delete.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(log_error!(
            "failed to delete container {}\nError: {}",
            container_id,
            err
        )
        .into());
    }

    // setup network
    log_info!("Reconfiguring network...");
    let service_net_config = NetworkConfig {
        mode: svc
            .network_mode
            .clone()
            .unwrap_or_else(|| "bridge".to_string()),
        ip_addres: svc.ip_address.clone().unwrap_or_default(),
        port: svc.ports.clone().unwrap_or_default(),
    };
    if let Err(e) = network::network_config_setup(container_id, &service_net_config).await {
        log_error!("Failed to setup network\nError: {}", e);
    }

    // mount overlayfs
    log_info!("Checking OverlayFS mount...");
    let upperdir = bundles.join("upper");
    let workdir = bundles.join("work");
    let mergeddir = bundles.join("rootfs");
    let merged_str = mergeddir.to_string_lossy();

    let is_mounted = std::fs::read_to_string("/proc/mounts")
        .map(|m| {
            m.lines()
                .any(|line| line.split_whitespace().nth(1) == Some(&merged_str))
        })
        .unwrap_or(false);

    if !is_mounted {
        log_info!("Remounting overlayFS...");
        let lowerdir = resolve_image(&svc.image).await?;
        let opts = format!(
            "lowerdir={},upperdir={},workdir={}",
            lowerdir.display(),
            upperdir.display(),
            workdir.display()
        );

        let output = Command::new("mount")
            .args(["-t", "overlay", "overlay", "-o", &opts])
            .arg(&mergeddir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(log_error!("Failed to mount OverlayFS: {}", stderr.trim()).into());
        }
        log_success!("Container {} sucess to remounted!", container_id);
    } else {
        log_info!("Container {} is already mounted, skipped.", container_id);
    }

    // restart container with create
    let mut create = container_cmd()?;
    let create_out = create
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("create")
        .arg(container_id)
        .arg("--bundle")
        .arg(&bundles)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if create_out.success() {
        match get_container_pid(container_id).await {
            Ok(pid) => {
                log_success!("Container {} is Restarted\nPID: {}", container_id, pid);
                if let Err(net_err) = network::connect_container_network(container_id, pid).await {
                    log_error!(
                        "Failed to connect network to container, but container has been still created\nError: {}",
                        net_err
                    );
                }
            }
            Err(e) => println!("Failed to get container PID\nError: {}", e),
        }
    } else {
        return Err(log_error!("Failed to create container.").into());
    }

    // start container
    log_info!("Turn on contianer '{}'...", &container_id);
    container_start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    // exec
    if !svc.command.is_empty() {
        log_info!("Restarting command application.");

        let mut exec_cmd = container_cmd()?;
        exec_cmd.arg("--root").arg(YOUKI_ROOT).arg("exec");

        if let Some(ref workdir) = svc.workdir {
            exec_cmd.arg("--cwd").arg(workdir);
        }

        for (k, v) in &svc.env {
            exec_cmd.arg("--env").arg(format!("{}={}", k, v));
        }

        exec_cmd
            .arg(&container_id)
            .args(&svc.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        exec_cmd
            .spawn()
            .map_err(|e| log_error!("Failed to spawn command application: {}", e))?;
    }

    log_success!("Your container has been restarted!.");

    Ok(())
}

// singgle container
pub async fn restart_container(container_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    log_info!("Restarting your container with reading the cache manifest lock");
    let manifest = ContainerManifest::load_container_lock(container_id)?;

    log_info!("Checking bundles folder..");
    let bundles = Path::new(BUNDLE_PATH).join(container_id);
    if !bundles.exists() {
        return Err(log_error!("Bundles folder is not found.").into());
    }

    // read db
    log_info!("Getting database information...");
    db::container::get_network(container_id)
        .map_err(|e| log_error!("failed to get database info, error: {}", e))?;

    // kill contianer
    log_info!("killing the container...");
    let signal = &format!("SIGKILL").to_string();
    if let Err(e) = runtime_kill(container_id, signal).await {
        log_error!("Container is not (Running) or is already stopped: {}", e);
    } else {
        log_info!("Container {} has been killed.", container_id);
    }

    // delete container
    log_info!("Recreating container instance...");
    let mut delete = container_cmd()?;
    delete
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("delete")
        .arg("--force")
        .arg(container_id);

    let output = delete.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(log_error!(
            "failed to delete container {}\nError: {}",
            container_id,
            err
        )
        .into());
    }

    // setup network
    log_info!("Reconfiguring network...");
    let service_net_config = NetworkConfig {
        mode: manifest
            .frozen_manifest
            .container
            .network_mode
            .clone()
            .unwrap_or_else(|| "bridge".to_string()),
        ip_addres: manifest
            .frozen_manifest
            .container
            .ip_address
            .clone()
            .unwrap_or_default(),
        port: manifest
            .frozen_manifest
            .container
            .ports
            .clone()
            .unwrap_or_default(),
    };
    if let Err(e) = network::network_config_setup(container_id, &service_net_config).await {
        log_error!("Failed to setup network\nError: {}", e);
    }

    // mount overlayfs
    log_info!("Checking OverlayFS mount...");
    let upperdir = bundles.join("upper");
    let workdir = bundles.join("work");
    let mergeddir = bundles.join("rootfs");
    let merged_str = mergeddir.to_string_lossy();

    let is_mounted = std::fs::read_to_string("/proc/mounts")
        .map(|m| {
            m.lines()
                .any(|line| line.split_whitespace().nth(1) == Some(&merged_str))
        })
        .unwrap_or(false);

    if !is_mounted {
        log_info!("Remounting overlayFS...");
        let lowerdir = resolve_image(&manifest.frozen_manifest.container.image).await?;
        let opts = format!(
            "lowerdir={},upperdir={},workdir={}",
            lowerdir.display(),
            upperdir.display(),
            workdir.display()
        );

        let output = Command::new("mount")
            .args(["-t", "overlay", "overlay", "-o", &opts])
            .arg(&mergeddir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(log_error!("Failed to mount OverlayFS: {}", stderr.trim()).into());
        }
        log_success!("Container {} sucess to remounted!", container_id);
    } else {
        log_info!("Container {} is already mounted, skipped.", container_id);
    }

    // restart container with create
    let mut create = container_cmd()?;
    let create_out = create
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("create")
        .arg(container_id)
        .arg("--bundle")
        .arg(&bundles)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if create_out.success() {
        match get_container_pid(container_id).await {
            Ok(pid) => {
                log_success!("Container {} is Restarted\nPID: {}", container_id, pid);
                if let Err(net_err) = network::connect_container_network(container_id, pid).await {
                    log_error!(
                        "Failed to connect network to container, but container has been still created\nError: {}",
                        net_err
                    );
                }
            }
            Err(e) => println!("Failed to get container PID\nError: {}", e),
        }
    }

    // start container
    log_info!("Turn on contianer '{}'...", &container_id);
    container_start(&container_id)
        .await
        .map_err(|e| e.to_string())?;

    // exec
    if !manifest.frozen_manifest.container.command.is_empty() {
        log_info!("Restarting command application.");

        let mut exec_cmd = container_cmd()?;
        exec_cmd.arg("--root").arg(YOUKI_ROOT).arg("exec");

        if let Some(ref workdir) = manifest.frozen_manifest.container.workdir {
            exec_cmd.arg("--cwd").arg(workdir);
        }

        for (k, v) in &manifest.frozen_manifest.container.env {
            exec_cmd.arg("--env").arg(format!("{}={}", k, v));
        }

        exec_cmd
            .arg(&container_id)
            .args(&manifest.frozen_manifest.container.command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        exec_cmd
            .spawn()
            .map_err(|e| log_error!("Failed to spawn command application: {}", e))?;
    }

    log_success!("Your container has been restarted!.");

    Ok(())
}
