pub async fn cummon_version() -> Result<(), Box<dyn std::error::Error>> {
    let xserve_version = "v0.1.1".to_string();
    let xserved_version = "v0.1.1".to_string();

    let status = "PRE-RELEASE".to_string();
    println!("xserve version: {}\nxserve-daemon version: {}\nstatus: {}", xserve_version, xserved_version, status);
    Ok(())
}