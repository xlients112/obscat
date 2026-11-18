use tokio::fs;

// GLOBAL PATH
pub const RUN_ROOT: &str = "/var/run/xserved";
pub const YOUKI_ROOT: &str = "/var/run/xserved/youki";
const BUNDLE_PATH: &str = "/var/lib/xserved/bundles";
const LOGS_PATH: &str = "/var/log/xserved";
const IMAGES_PATH: &str = "/var/lib/xserved/images";

pub async fn init_system() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //init root runtime
    fs::create_dir_all(RUN_ROOT).await?;
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