use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{fs, process::Command, time::sleep};

use crate::{
    internal_proto::NetworkConfig,
    log_error, log_success,
    runtime::{container_internal::container_cmd, network},
    utils::{BUNDLE_PATH, IMAGES_PATH, LOGS_PATH, YOUKI_ROOT},
};
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
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = container_cmd()?;
    cmd.args(["--root", YOUKI_ROOT, "state"]).arg(container_id);

    let output = &cmd.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        log_error!("Failed to get container PID\nError: {}", err);
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let state_data: YoukiState = serde_json::from_str(&json_str)?;

    if let Some(pid_num) = state_data.pid {
        Ok(pid_num)
    } else {
        Err(log_error!(
            "Container {} has no have pid active (die or not running)",
            container_id
        )
        .into())
    }
}

// runtime spec
pub async fn runtime_spec(
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bundle = Path::new(BUNDLE_PATH).join(container_id);

    let mut spec = container_cmd()?;
    spec.arg("spec").current_dir(&bundle);

    let output = spec.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(Box::<dyn std::error::Error + Send + Sync>::from(
            log_error!("Failed to create config.json\nError: {}", err),
        ));
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
        for ns in namespaces.iter_mut() {
            if ns.get("type").and_then(|t| t.as_str()) == Some("network") {
                ns["path"] = Value::String(format!("/var/run/netns/{}", container_id));
            }
        }
    }

    let ready_json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, ready_json).await?;
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
        .map_err(|_| log_error!("Image not found or you not image pull before (index.json not found): try: obscat image pull frist"))?;

    let records: Vec<ImageRecord> = serde_json::from_str(&content)?;
    let exists = records
        .iter()
        .any(|r| r.name == scope_image && r.tag == tag);

    if !exists {
        return Err(log_error!(
            "Image '{}:{}' image not found in local store. Pull frist with `obscat image pull {}`.",
            scope_image,
            tag,
            image
        )
        .into());
    }

    let safe_dir_name = format!("{}_{}", scope_image.replace('/', "_"), tag);
    let image_rootfs = Path::new(IMAGES_PATH).join(safe_dir_name).join("rootfs");

    if !fs::try_exists(&image_rootfs).await.unwrap_or(false) {
        return Err(log_error!("rootfs directory '{}' is broken or missing.", image).into());
    }

    Ok(image_rootfs)
}

pub async fn runtime_create(
    container_id: &str,
    image: &str,
    net_config: &NetworkConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // setup container before create container
    if let Err(e) = network::network_config_setup(container_id, net_config).await {
        log_error!("Failed to setup network\nError: {}", e);
    }

    // create overlayFS dir
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

    // creating spec for container
    runtime_spec(container_id).await?;

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
                log_success!("Container {} is created\nPID: {}", container_id, pid);
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

    Ok(())
}

// runtime delete
pub async fn runtime_delete(
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bundle = Path::new(BUNDLE_PATH).join(container_id);
    let merged_rootfs = bundle.join("rootfs");

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

    // umount overlayfs
    if fs::try_exists(&merged_rootfs).await.unwrap_or(false) {
        let umount_out = Command::new("umount").arg(&merged_rootfs).output().await?;

        if umount_out.status.success() {
            log_success!(
                "OverlayFS in {:?} has been sucess to unmount.",
                merged_rootfs
            );
        } else {
            let stderr = String::from_utf8_lossy(&umount_out.stderr);
            // Jika error bukan karena "not mounted", hentikan penghapusan demi keamanan data!
            if !stderr.contains("not mounted") {
                log_error!(
                    "Failed unmount OverlayFS (canceled delete folder for security check): {}",
                    stderr.trim()
                );
                return Err(log_error!("umount Failed: {}", stderr.trim()).into());
            }
        }
    }

    // clean network
    if let Err(e) = network::clean_network(container_id).await {
        println!("Failed to cleaning network\nError: {}", e)
    }

    // clean bundles file
    if tokio::fs::try_exists(&bundle).await? {
        tokio::fs::remove_dir_all(&bundle).await?;
    }

    // clean log
    let log_path = format!("{}/{}", LOGS_PATH, container_id);
    if std::path::Path::new(&log_path).exists() {
        let _ = tokio::fs::remove_file(log_path).await;
    }

    Ok(())
}

// runtime state
pub async fn state(container_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut state = container_cmd()?;
    state
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    let state_output = state.output().await?;
    if state_output.status.success() {
        let state_json = String::from_utf8_lossy(&state_output.stdout);
        println!("Container status: {}", state_json);
    } else {
        let state_err = String::from_utf8_lossy(&state_output.stderr);
        log_error!(
            "Failed to state container: {} error\nError: {}",
            container_id,
            state_err
        );
    }

    Ok(())
}

// runtime list
pub async fn runtime_list() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut status_map: HashMap<String, String> = HashMap::new();
    let output = container_cmd()?
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("list")
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
            for c in list {
                if let (Some(id), Some(status)) = (
                    c.get("id").and_then(|v| v.as_str()),
                    c.get("status").and_then(|v| v.as_str()),
                ) {
                    // Kapitalisasi huruf depan biar rapi (misal "running" -> "Running")
                    let mut chars = status.chars();
                    let formatted_status = match chars.next() {
                        None => "Stopped".to_string(),
                        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                    };
                    status_map.insert(id.to_string(), formatted_status);
                }
            }
        }
    }

    let items = crate::runtime::db::list_all_items()
        .map_err(|e| log_error!("Failed to read containers from DB: {}", e))?;

    if items.is_empty() {
        return Ok("No containers found.\n".to_string());
    }

    let truncate = |s: &str, max: usize| -> String {
        if s.chars().count() > max {
            format!("{}..", s.chars().take(max - 2).collect::<String>())
        } else {
            s.to_string()
        }
    };

    let mut table = format!(
        "{:<18} {:<22} {:<18} {:<10} {:<15} {}\n",
        "ID", "NAME (DEPLOY)", "IMAGE", "STATUS", "IP ADDRESS", "PORTS"
    );

    for item in items {
        let status = status_map
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| "Stopped".to_string());

        let ip = if item.ip_address.trim().is_empty() {
            "-"
        } else {
            &item.ip_address
        };
        let port = if item.port.trim().is_empty() {
            "-"
        } else {
            &item.port
        };

        table.push_str(&format!(
            "{:<18} {:<22} {:<18} {:<10} {:<15} {}\n",
            truncate(&item.id, 17),
            truncate(&item.display_name, 21),
            truncate(&item.image, 17),
            status,
            ip,
            port
        ));
    }

    Ok(table)
}

// container start
pub async fn container_start(
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_dir = Path::new(YOUKI_ROOT).join(&container_id);
    // check status state
    if state_dir.exists() {
        let mut state = container_cmd()?;
        state
            .arg("--root")
            .arg(YOUKI_ROOT)
            .arg("state")
            .arg(&container_id);

        let state_out = state.output().await?;

        if state_out.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&state_out.stdout) {
                match json.get("status").and_then(|s| s.as_str()) {
                    Some("running") => {
                        log_error!("Container '{}' is already running", container_id);
                        return Ok(());
                    }

                    Some("stopped") => {
                        let _ = container_cmd()?
                            .args(["--root", YOUKI_ROOT, "delete", "--force", container_id])
                            .output()
                            .await?;

                        let bundle_path = Path::new(BUNDLE_PATH).join(container_id);
                        let create_out = container_cmd()?
                            .args(["--root", YOUKI_ROOT, "create", "--bundle"])
                            .arg(&bundle_path)
                            .arg(container_id)
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .output()
                            .await?;

                        if !create_out.status.success() {
                            let err = String::from_utf8_lossy(&create_out.stderr);
                            return Err(log_error!(
                                "Failed to recreate container '{}': {}",
                                container_id,
                                err
                            )
                            .into());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let output = container_cmd()?
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("start")
        .arg(container_id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await?;

    if output.status.success() {
        log_success!("Container '{}' has been started (start)", container_id);
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(log_error!("Failed to start container '{}': {},", container_id, err).into())
    }
}

pub async fn runtime_kill(
    container_id: &str,
    signal: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut kill = container_cmd()?;
    kill.arg("--root")
        .arg(YOUKI_ROOT)
        .arg("kill")
        .arg(container_id)
        .arg(signal);

    sleep(Duration::from_secs(1)).await;

    let output = kill.output().await?;
    if output.status.success() {
        log_success!(
            "container: {} has been succes killed {}",
            container_id,
            signal
        );
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log_error!("failed to kill container: {}\nError: {}", container_id, err);
    }

    // update state
    let mut state = container_cmd()?;
    state
        .arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    Ok(())
}
