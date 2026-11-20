use std::{fs, path::Path};

pub async fn cummon_version() -> Result<(), Box<dyn std::error::Error>> {
    let obscat_version = "v0.1.1".to_string();
    let obscatd_version = "v0.1.1".to_string();

    let status = "PRE-RELEASE".to_string();
    println!(
        "Obscat version: {}\ndaemon version: {}\nstatus: {}",
        obscat_version, obscatd_version, status
    );
    Ok(())
}

pub async fn init_deployment(deploy_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let deploy_path = Path::new(deploy_name);
    if deploy_path.exists() {
        return Err(format!(
            "Deployment folder {} is already exists",
            deploy_name
        ).into());
    }

    let clean_name = deploy_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(deploy_name);

    let service_dir = deploy_path.join("service").join("yourapps");
    fs::create_dir_all(&service_dir)?;

    let manifest_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[service.yourapps]
image = "busybox:latest"
destination = "/www"
workdir = "/www"
ports = "8080:80"
command = ["/bin/httpd", "-f", "-p", "80", "-h", "/www"]

[service.yourapps.env]
APP_ENV = "production"
"#,
        clean_name
    );

    fs::write(deploy_path.join("obscat.toml"), manifest_content)?;

    let sample_html = format!(
        "<h1>Hello from {}!</h1>\n<p>Running inside obscat container.</p>\n",
        clean_name
    );
    fs::write(service_dir.join("index.html"), sample_html)?;

    println!("Deployment folder has been sucess to create.");
    println!("go to inside Deployment folder, and edit manifest.toml config");
    println!("or.. add new folder application on {}/service to create new application container instance.", clean_name);
    println!("NOTE: nothing to build, {} is only your deployment space folder to manage your application container. read docs in github", clean_name);


    Ok(())
}
