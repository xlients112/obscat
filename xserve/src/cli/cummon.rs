pub async fn cummon_version() -> Result<(), Box<dyn std::error::Error>> {
    let version = "v0.1.0".to_string();
    let status = "beta version".to_string();
    println!("version: {}\nstatus: {}", version, status);
    Ok(())
}