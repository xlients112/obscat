use std::{
    path::{Path, PathBuf}, process::Stdio, time::Duration,
};

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{fs, process::Command, time::sleep};

use crate::{
    internal_proto::NetworkConfig, runtime::{logging::LogsCollector, network, utils::{BUNDLE_PATH, IMAGES_PATH, LOGS_PATH, YOUKI_ROOT}},
};

// GLOBAL PATH
// const IMAGE_STORE_PATH: &str = "/var/lib/xserved/images";
// const BUNDLE_PATH: &str = "/var/lib/xserved/bundles";
// const LOGS_PATH: &str = "/var/log/xserved";

// youki state
#[derive(Debug, Deserialize)]
pub struct YoukiState {
    pub id: String,
    pub status: String,
    pub pid: Option<i32>,
    pub bundle: String,
}

#[derive(Deserialize, Debug)]
struct ImageRecord {
    name: String,
    tag: String,
    #[allow(dead_code)]
    size: u64,
    #[allow(dead_code)]
    layers: Vec<String>,
}

// get container pid
pub async fn get_container_pid(
    container_id: &str,
    log: &mut LogsCollector,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = Command::new("youki");
    cmd.arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    let output = cmd.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        log.error(format!("Failed to get container PID\nError: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let state_data: YoukiState = serde_json::from_str(&json_str)?;

    if let Some(pid_num) = state_data.pid {
        Ok(pid_num)
    } else {
        Err(format!(
            "Container {} has no have pid active (die or not running)",
            container_id
        )
        .into())
    }
}

// runtime spec
pub async fn runtime_spec(
    container_id: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bundle = Path::new(BUNDLE_PATH).join(container_id);

    let mut spec = Command::new("youki");
    spec.arg("spec").current_dir(&bundle);

    let output = spec.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
            "Failed to create config.json\nError: {}",
            err
        )));
    }

    // read json
    let config_path = bundle.join("config.json");
    let raw_json = fs::read_to_string(&config_path).await?;
    let mut config: Value = serde_json::from_str(&raw_json)?;

    config["hostname"] = Value::String(container_id.to_string());

    if let Some(readonly) = config.pointer_mut("/root/readonly") {
        *readonly = Value::Bool(false);
    }
    if let Some(process) = config.get_mut("process") {
        process["terminal"] = Value::Bool(false);
        process["args"] = json!(["sh", "-c", "while true; do sleep 3600; done"]);

        // D. Tambahkan Linux Capabilities standar agar apt/nginx/su tidak kena error permission
        let default_caps = json!([
            "CAP_AUDIT_WRITE",
            "CAP_KILL",
            "CAP_NET_BIND_SERVICE",
            "CAP_SETUID",
            "CAP_SETGID",
            "CAP_CHOWN",
            "CAP_FOWNER",
            "CAP_DAC_OVERRIDE",
            "CAP_NET_RAW"
        ]);

        if let Some(caps) = process.get_mut("capabilities") {
            caps["bounding"] = default_caps.clone();
            caps["effective"] = default_caps.clone();
            caps["permitted"] = default_caps;
        }
    }

    if let Some(mounts) = config.pointer_mut("/mounts").and_then(|m| m.as_array_mut()) {
        mounts.push(json!({
            "destination": "/etc/resolv.conf",
            "type": "bind",
            "source": "/etc/resolv.conf",
            "options": ["rbind", "ro"]
        }));
        mounts.push(json!({
            "destination": "/etc/hosts",
            "type": "bind",
            "source": "/etc/hosts",
            "options": ["rbind", "ro"]
        }));
    }

    if let Some(namespaces) = config
        .pointer_mut("/linux/namespaces")
        .and_then(|n| n.as_array_mut())
    {
        namespaces.retain(|ns| ns.get("type").and_then(|t| t.as_str()) != Some("network"));
    }

    let ready_json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, ready_json).await?;

    log.info("Success to create config.json");
    Ok(())
}

// runtime create
async fn resolve_image(image: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
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

    let index_path = Path::new(IMAGES_PATH).join("index.json");
    let content = fs::read_to_string(&index_path)
        .await
        .map_err(|_| "Image not found or you not image pull before (index.json not found): try: xserve image pull frist")?;

    let records: Vec<ImageRecord> = serde_json::from_str(&content)?;
    let exists = records
        .iter()
        .any(|r| r.name == scope_image && r.tag == tag);

    if !exists {
        return Err(format!(
            "Image '{}:{}' image not found in local store. Pull frist with `xserve image pull {}`.",
            scope_image, tag, image
        )
        .into());
    }

    let safe_dir_name = format!("{}_{}", scope_image.replace('/', "_"), tag);
    let image_rootfs = Path::new(IMAGES_PATH)
        .join(safe_dir_name)
        .join("rootfs");

    if !fs::try_exists(&image_rootfs).await.unwrap_or(false) {
        return Err(format!("rootfs directory '{}' is broken or missing.", image).into());
    }

    Ok(image_rootfs)
}

pub async fn runtime_create(
    container_id: &str,
    image: &str,
    net_config: &NetworkConfig,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // setup container before create container
    if let Err(e) = network::network_config_setup(container_id, net_config).await {
        log.error(format!("Failed to setup network\nError: {}", e));
    }

    // create overlayFS dir
    log.info("Creating overlayFS directory...");
    let bundles = Path::new(BUNDLE_PATH).join(container_id);
    fs::create_dir_all(&bundles).await?;
    let lowerdir = resolve_image(image).await?;
    let upperdir = bundles.join("upper");
    let workdir = bundles.join("work");
    let mergeddir = bundles.join("rootfs");

    fs::create_dir_all(&upperdir).await?;
    fs::create_dir_all(&workdir).await?;
    fs::create_dir_all(&mergeddir).await?;

    // mount overlayfs
    log.info("Mounting overlayFS...");
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
        return Err(format!("Gagal mount OverlayFS: {}", stderr.trim()).into());
    }

    log.info("Creating spec config.json for container..");
    runtime_spec(container_id, log).await?;

    log.info("creating container instance...");
    let mut create = Command::new("youki");
    create
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("create")
        .arg(container_id)
        .arg("--bundle")
        .arg(bundles);

    if !output.status.success() {
        log.info("container has been succes created");
        match get_container_pid(container_id, log).await {
            Ok(pid) => {
                log.info(format!("Container {} is created\nPID: {}", container_id, pid));
                if let Err(net_err) = network::connect_container_network(container_id, pid).await {
                    log.error(format!(
                        "Failed to connect network to container, but container has been still created\nError: {}",
                        net_err
                    ));
                }
            }
            Err(e) => log.error(format!("Failed to get container PID\nError: {}", e)),
        }
    }

    Ok(())
}

// runtime delete
pub async fn runtime_delete(
    container_id: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log.info(format!(
        "Start deleting container from host\nContainerID: {}",
        container_id
    ));
    let bundle = Path::new(BUNDLE_PATH).join(container_id);
    let merged_rootfs = bundle.join("rootfs");

    let mut delete = Command::new("youki");
    delete
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("delete")
        .arg("--force")
        .arg(container_id);

    let output = delete.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "failed to delete container {}\nError: {}",
            container_id, err
        )
        .into());
    }

    // umount overlayfs
    if fs::try_exists(&merged_rootfs).await.unwrap_or(false) {
        let umount_out = Command::new("umount").arg(&merged_rootfs).output().await?;

        if umount_out.status.success() {
            log.info(format!(
                "OverlayFS in {:?} has been sucess to unmount.",
                merged_rootfs
            ));
        } else {
            let stderr = String::from_utf8_lossy(&umount_out.stderr);
            // Jika error bukan karena "not mounted", hentikan penghapusan demi keamanan data!
            if !stderr.contains("not mounted") {
                log.error(format!(
                    "Failed unmount OverlayFS (canceled delete folder for security check): {}",
                    stderr.trim()
                ));
                return Err(format!("umount Failed: {}", stderr.trim()).into());
            }
        }
    }

    // clean network
    log.info("cleanup network interface..");
    if let Err(e) = network::clean_network(container_id).await {
        log.error(format!("Failed to cleaning network\nError: {}", e))
    }

    // clean bundles file
    if tokio::fs::try_exists(&bundle).await? {
        tokio::fs::remove_dir_all(&bundle).await?;
    }

    // clean log
    log.info("cleanup log file..");
    let log_path = format!("{}/{}", LOGS_PATH, container_id);
    if std::path::Path::new(&log_path).exists() {
        let _ = tokio::fs::remove_file(log_path).await;
        log.info("Log file has been cleanup.");
    }

    log.info(format!("Sucess to delete container: {}", container_id));
    Ok(())
}

// runtime state
pub async fn state(
    container_id: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = Command::new("youki");
    state
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    let state_output = state.output().await?;
    if state_output.status.success() {
        let state_json = String::from_utf8_lossy(&state_output.stdout);
        log.info(format!("Container status: {}", state_json));
    } else {
        let state_err = String::from_utf8_lossy(&state_output.stderr);
        log.error(format!(
            "Failed to state container: {} error\nError: {}",
            container_id, state_err
        ));
    }

    Ok(())
}
// rountime run
pub async fn container_run(
    container_id: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error>> {
    //deleted state in root youki folder
    let _ = Command::new("youki")
        .args(["--root", YOUKI_ROOT, "delete", container_id])
        .output()
        .await;

    let bundle = &format!("{}/{}", BUNDLE_PATH, container_id);
    let mut run = Command::new("youki");
    run.arg("--root")
        .arg(YOUKI_ROOT)
        .arg("run")
        .arg("--detach")
        .arg("--bundle")
        .arg(bundle)
        .arg(container_id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let output = run.output().await?;
    if output.status.success() {
        log.info("succes to run container\n");
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log.error(format!(
            "failed to run container: {}\nError: {}",
            container_id, err
        ));
    }
    Ok(())
}

pub async fn runtime_kill(
    container_id: &str,
    signal: &str,
    log: &mut LogsCollector,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut kill = Command::new("youki");
    kill.arg("--root")
        .arg(YOUKI_ROOT)
        .arg("kill")
        .arg(container_id)
        .arg(signal);

    sleep(Duration::from_secs(1)).await;

    let output = kill.output().await?;
    if output.status.success() {
        log.error(format!(
            "container: {} has been succes killed {}",
            container_id, signal
        ));
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log.error(format!(
            "failed to kill container: {}\nError: {}",
            container_id, err
        ));
    }

    // update state
    let mut state = Command::new("youki");
    state
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    Ok(())
}