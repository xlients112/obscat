use tokio::fs;

use crate::utils::{BUNDLE_PATH, IMAGES_PATH, LOGS_PATH, PID_FILE, RUN_ROOT, YOUKI_ROOT};

pub async fn init_system() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //init root runtime
    fs::create_dir_all(RUN_ROOT).await?;
    let pid = std::process::id();
    tokio::fs::write(PID_FILE, pid.to_string()).await?;
    // init youki runtime
    fs::create_dir_all(YOUKI_ROOT).await?;

    // init bundle global path
    fs::create_dir_all(BUNDLE_PATH).await?;

    // init logs path
    fs::create_dir_all(LOGS_PATH).await?;
    // images path
    fs::create_dir_all(IMAGES_PATH).await?;
    Ok(())
}