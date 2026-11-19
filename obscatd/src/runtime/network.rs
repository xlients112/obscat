use std::hash::{DefaultHasher, Hash, Hasher};

use crate::internal_proto::NetworkConfig;
use tokio::process::Command;

fn short_veth_id(container_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    container_id.hash(&mut hasher);

    format!("{:06x}", hasher.finish() & 0xFFFFFF)
}

fn allocated_ip(container_id: &str, request_ip: &str) -> String {
    if !request_ip.trim().is_empty() {
        return request_ip.trim().to_string();
    }
    let mut hasher = DefaultHasher::new();
    container_id.hash(&mut hasher);
    let octet = ((hasher.finish() % 250) + 2) as u8;
    format!("172.19.0.{}", octet)
}

pub async fn init_global_bridge() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Checking global bridge (xsvd0)...");

    let check_bridge = Command::new("ip")
        .args(["link", "show", "xsvd0"])
        .output()
        .await?;

    if !check_bridge.status.success() {
        println!("Global bridge (xsvd0) is already active and ready to use.");
        Command::new("ip")
            .args(["link", "add", "xsvd0", "type", "bridge"])
            .output()
            .await?;

        Command::new("ip")
            .args(["addr", "add", "172.19.0.1/16", "dev", "xsvd0"])
            .output()
            .await?;

        Command::new("ip")
            .args(["link", "set", "xsvd0", "up"])
            .output()
            .await?;
    }

    println!("Global bridge not found. Creating new xsvd0...");
    if let Err(e) = std::fs::write("/proc/sys/net/ipv4/ip_forward", "1") {
        eprintln!(
            "Warning: Failed to create IP Forwarding (you must have root access): {}",
            e
        );
    }

    let check_nat = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-C",
            "POSTROUTING",
            "-s",
            "172.19.0.0/16",
            "!",
            "-o",
            "xsvd0",
            "-j",
            "MASQUERADE",
        ])
        .output()
        .await?;

    if !check_nat.status.success() {
        Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-s",
                "172.19.0.0/16",
                "!",
                "-o",
                "xsvd0",
                "-j",
                "MASQUERADE",
            ])
            .output()
            .await?;
    }

    println!("Global bridge (xsvd0) is active at 172.19.0.1/16");
    Ok(())
}

pub async fn network_config_setup(
    container_id: &str,
    config: &NetworkConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if config.mode != "bridge" {
        return Ok(());
    }

    let vid = short_veth_id(container_id);
    let veth_host = format!("vh_{}", vid); // Sisi host (menempel ke xsvd0)
    let veth_container = format!("vc_{}", vid);

    let _ = Command::new("ip")
        .args(["link", "delete", &veth_host])
        .output()
        .await;

    let out = Command::new("ip")
        .args([
            "link",
            "add",
            &veth_host,
            "type",
            "veth",
            "peer",
            "name",
            &veth_container,
        ])
        .output()
        .await?;

    if !out.status.success() {
        return Err(format!(
            "Failed to create veth: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }

    Command::new("ip")
        .args(["link", "set", &veth_host, "master", "xsvd0"])
        .output()
        .await?;

    Command::new("ip")
        .args(["link", "set", &veth_host, "up"])
        .output()
        .await?;

    if !config.port.trim().is_empty() {
        let container_ip = allocated_ip(container_id, &config.ip_addres);
        setup_port_forwarding(&container_ip, &config.port).await?;
    }

    Ok(())
}

pub async fn connect_container_network(
    container_id: &str,
    pid: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let vid = short_veth_id(container_id);
    let veth_container = format!("vc_{}", vid);
    let pid_str = pid.to_string();
    let container_ip = allocated_ip(container_id, "");

    let out_move = Command::new("ip")
        .args(["link", "set", &veth_container, "netns", &pid_str])
        .output()
        .await?;

    if !out_move.status.success() {
        return Err(format!(
            "Failed to overide {} to netns {}: {}",
            veth_container,
            pid_str,
            String::from_utf8_lossy(&out_move.stderr)
        )
        .into());
    }

    Command::new("nsenter")
        .args(["-t", &pid_str, "-n", "ip", "link", "set", "lo", "up"])
        .output()
        .await?;

    Command::new("nsenter")
        .args([
            "-t",
            &pid_str,
            "-n",
            "ip",
            "link",
            "set",
            &veth_container,
            "name",
            "eth0",
        ])
        .output()
        .await?;

    Command::new("nsenter")
        .args(["-t", &pid_str, "-n", "ip", "link", "set", "eth0", "up"])
        .output()
        .await?;

    let cidr_ip = format!("{}/16", container_ip);
    Command::new("nsenter")
        .args([
            "-t", &pid_str, "-n", "ip", "addr", "add", &cidr_ip, "dev", "eth0",
        ])
        .output()
        .await?;

    Command::new("nsenter")
        .args([
            "-t",
            &pid_str,
            "-n",
            "ip",
            "route",
            "add",
            "default",
            "via",
            "172.19.0.1",
        ])
        .output()
        .await?;

    println!(
        "Container network '{}' active with IP: {}",
        container_id, container_ip
    );
    Ok(())
}

pub async fn setup_port_forwarding(
    container_ip: &str,
    port_mapping: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host_port, container_port) = port_mapping
        .split_once(':')
        .ok_or("Format port must be like host_port:container_port (example: 8080:80)")?;

    let dest = format!("{}:{}", container_ip, container_port);
    Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-A",
            "PREROUTING",
            "-p",
            "tcp",
            "--dport",
            host_port,
            "-j",
            "DNAT",
            "--to-destination",
            &dest,
        ])
        .output()
        .await?;

    Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-A",
            "OUTPUT",
            "-p",
            "tcp",
            "-d",
            "127.0.0.1",
            "--dport",
            host_port,
            "-j",
            "DNAT",
            "--to-destination",
            &dest,
        ])
        .output()
        .await?;

    println!("Port forwarding active: :{} -> {}", host_port, dest);
    Ok(())
}

pub async fn clean_network(
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let vid = short_veth_id(container_id);
    let veth_host = format!("vh_{}", vid);
    println!("Cleanup network for container veth: {}", veth_host);

    let check = Command::new("ip")
        .args(["link", "show", &veth_host])
        .output()
        .await?;

    if check.status.success() {
        Command::new("ip")
            .arg("link")
            .arg("delete")
            .arg(&veth_host)
            .output()
            .await?;
        println!("interface netwok {} has been deleted from host", veth_host);
    }
    Ok(())
}
