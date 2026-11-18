use crate::internal_proto::NetworkConfig;
use tokio::process::Command;

/// Mempersiapkan interface Bridge Utama (xsvd0) di Linux Host
pub async fn init_global_bridge() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Mengecek ketersediaan global bridge (xsvd0)...");

    // Cek apakah bridge xsvd0 sudah ada di OS agar tidak bentrok
    let check_bridge = Command::new("ip")
        .args(["link", "show", "xsvd0"])
        .output()
        .await?;

    if check_bridge.status.success() {
        println!("Global bridge (xsvd0) sudah aktif dan siap digunakan.");
        return Ok(());
    }

    println!("Global bridge tidak ditemukan. Membuat xsvd0 baru...");

    //  Buat interface bertipe bridge
    Command::new("ip")
        .args(["link", "add", "xsvd0", "type", "bridge"])
        .output()
        .await?;

    // Berikan IP Gateway default untuk jaringan kontainer (Misal: 172.19.0.1)
    Command::new("ip")
        .args(["addr", "add", "172.19.0.1/16", "dev", "xsvd0"])
        .output()
        .await?;

    // Nyalakan interface bridge-nya
    Command::new("ip")
        .args(["link", "set", "xsvd0", "up"])
        .output()
        .await?;

    // Aktifkan IP Forwarding di Kernel Linux agar kontainer bisa tembus internet luar
    // Menggunakan library std::fs bawaan Rust untuk menulis ke kernel /proc
    if let Err(e) = std::fs::write("/proc/sys/net/ipv4/ip_forward", "1") {
        eprintln!(
            "Warning: Gagal mengaktifkan IP Forwarding kernel (Butuh hak akses root): {}",
            e
        );
    }

    println!("Global bridge (xsvd0) berhasil diinisialisasi pada IP 172.19.0.1");
    Ok(())
}

pub async fn network_config_setup(
    container_id: &str,
    config: &NetworkConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if config.mode != "bridge" {
        return Ok(());
    }

    println!("Ready to set config network {}", container_id);
    let veth_host = format!(
        "veth_{}",
        &container_id[..std::cmp::min(5, container_id.len())]
    );
    let veth_container = format!(
        "veth_container_{}",
        &container_id[..std::cmp::min(5, container_id.len())]
    );

    let mut cmd = Command::new("ip");
    cmd.arg("link");
    cmd.arg("add");
    cmd.arg(&veth_host);
    cmd.arg("type");
    cmd.arg("veth");
    cmd.arg("peer");
    cmd.arg("name");
    cmd.arg(&veth_container);
    cmd.output().await?;

    // sodokan veth host ke bridge utama
    let mut cmd_bridge = Command::new("ip");
    cmd_bridge.arg("link");
    cmd_bridge.arg("set");
    cmd_bridge.arg(&veth_host);
    cmd_bridge.arg("master");
    cmd_bridge.arg("xsvd0");
    cmd_bridge.output().await?;

    let mut cmd_up = Command::new("ip");
    cmd_up.arg("link");
    cmd_up.arg("set");
    cmd_up.arg(&veth_host);
    cmd_up.arg("up");
    cmd_up.output().await?;

    Ok(())
}

pub async fn connect_container_network(
    container_id: &str,
    pid: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let veth_container = format!(
        "veth_{}",
        &container_id[..std::cmp::min(5, container_id.len())]
    );

    println!(
        "ready to connect interface {} to pid {}",
        veth_container, pid
    );

    let mut cmd_move = Command::new("ip");
    cmd_move
        .arg("link")
        .arg("set")
        .arg(&veth_container)
        .arg("nets")
        .arg(pid.to_string());

    let output = cmd_move.output().await?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        println!(
            "Failed to move veth to container namespaces\nError: {}",
            err
        );
    }

    // penamaan baru ke interface container dan beri ip
    let mut cmd_rename = Command::new("nsenter");
    cmd_rename
        .arg("-t")
        .arg(&pid.to_string())
        .arg("-n")
        .arg("ip")
        .arg("link")
        .arg("set")
        .arg(&veth_container)
        .arg("name")
        .arg("eth0");

    cmd_rename.output().await?;

    // nyalakan interface eth0
    let mut cmd_eth_up = Command::new("nsenter");
    cmd_eth_up.args([
        "-t",
        &pid.to_string(),
        "-n",
        "ip",
        "link",
        "set",
        "eth0",
        "up",
    ]);
    cmd_eth_up.output().await?;

    // 4. BERI IP KE 'eth0' (Contoh IP Statis dulu, kelak bisa Anda buat dinamis lewat database IPAM)
    let mut cmd_ip = Command::new("nsenter");
    cmd_ip.args([
        "-t",
        &pid.to_string(),
        "-n",
        "ip",
        "addr",
        "add",
        "172.19.0.2/16",
        "dev",
        "eth0",
    ]);
    cmd_ip.output().await?;

    // 5. SET DEFAULT GATEWAY AGAR KONTAINER BISA MENGAKSES INTERNET LUAR VIA BRIDGE HOST
    let mut cmd_gw = Command::new("nsenter");
    cmd_gw.args([
        "-t",
        &pid.to_string(),
        "-n",
        "ip",
        "route",
        "add",
        "default",
        "via",
        "172.19.0.1",
    ]);
    cmd_gw.output().await?;

    println!(
        "Jaringan kontainer {} berhasil dikonfigurasi (IP: 172.19.0.2)",
        container_id
    );
    Ok(())
}

pub async fn clean_network(container_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let veth_host = format!("veth_{}", &container_id[..std::cmp::min(5, container_id.len())]);
    println!("Cleanup network for container veth: {}", veth_host);
    // Periksa dulu apakah interface veth ini masih ada di host
    let check = Command::new("ip").args(["link", "show", &veth_host]).output().await?;

    if check.status.success() {
        let mut cmd_del = Command::new("ip");
        cmd_del.arg("link")
        .arg("delete")
        .arg(&veth_host);
    println!("interface netwok {} has been deleted from host", veth_host);
    }
    Ok(())
}
