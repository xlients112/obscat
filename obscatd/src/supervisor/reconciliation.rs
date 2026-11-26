use std::error::Error;

use crate::{
    log_error, log_info, runtime::{
        container_internal::container_cmd,
        db::list_all_for_reconciliation,
        internal::{YoukiState},
    }, supervisor::restart::restart, utils::YOUKI_ROOT,
};

pub async fn init_reconcile() -> Result<(), Box<dyn Error + Send + Sync>> {
    log_info!("Reconciliation all container...");

    let container = list_all_for_reconciliation().unwrap_or_default();
    for c in container {
        if c.state.eq_ignore_ascii_case("running") {
            let is_alive = check_actual(&c.id).await;
            if !is_alive {
                log_info!("Container is not synchronized in actuals state, restarting...");
                if let Err(e) = restart(&c.id).await {
                    log_error!("Failed to Reconciliation container {}, Error: {}", c.id, e);
                }
            } else {
                log_info!("The container is now synchronized with the actuals state.");
            }
        }
    }
    log_info!("All container is has been sucess to synchronized.");
    Ok(())
}

async fn check_actual(container_id: &str) -> bool {
    let mut cmd = match container_cmd() {
        Ok(c) => c,
        Err(_) => return false,
    };

    cmd.arg("--root")
        .arg(YOUKI_ROOT)
        .arg("state")
        .arg(container_id);

    if let Ok(output) = cmd.output().await {
        if output.status.success() {
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(state_data) = serde_json::from_str::<YoukiState>(&json_str) {
                    return state_data.status == "running" || state_data.status == "created";
                }
            }
        }
    }
    false
}
