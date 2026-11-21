use std::path::Path;

// use crate::grpcutils;
use crate::obscat_proto::container_service_client::ContainerServiceClient;
use crate::obscat_proto::{
    CreateRequest, DeleteRequest, KillRequest, ListRequest, RunRequest, StateRequest, StreamLogsRequest,
};
use tonic::{Request, transport::Channel};

pub async fn daemon_debug(
    client: &mut ContainerServiceClient<Channel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Attached to obscat live debug stream (Ctrl+C to exit)...");
    let mut stream = client
        .stream_logs(Request::new(StreamLogsRequest {}))
        .await?
        .into_inner();

    while let Some(log_item) = stream.message().await? {
        println!("{}", log_item.content);
    }

    Ok(())
}

// create request
pub async fn daemon_create(
    client: &mut ContainerServiceClient<Channel>,
    container_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting build container...");

    let raw_path = Path::new(container_path);
    let abs_path = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };
    if !abs_path.exists() {
        return Err(format!("Folder is not found: {:?}", abs_path).into());
    }
    let final_package_path = abs_path.to_string_lossy().to_string();

    let request = Request::new(CreateRequest {
        container_id: final_package_path.to_string(),
    });

    match client.create(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("create request success\nmessage: {}", res.message);
            } else {
                println!("failed to send request\nmessage: {}", res.message);
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failes\nError: {}", e.message());
        }
    }

    Ok(())
}

// delete request
pub async fn daemon_delete(
    client: &mut ContainerServiceClient<Channel>,
    container_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting to delete container...");

    let raw_path = Path::new(container_path);
    let abs_path = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };
    if !abs_path.exists() {
        return Err(format!("Folder is not found: {:?}", abs_path).into());
    }
    let final_package_path = abs_path.to_string_lossy().to_string();

    let request = Request::new(DeleteRequest {
        container_id: final_package_path.to_string(),
    });

    match client.delete(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("success to send request\nmessage: {}", res.message)
            } else {
                println!("failed to send request\nmessage: {}", res.message)
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failes\nError: {}", e.message());
        }
    }
    Ok(())
}

// state request
pub async fn daemon_state(
    client: &mut ContainerServiceClient<Channel>,
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request = Request::new(StateRequest {
        container_id: container_id.to_string(),
    });

    match client.state(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("success to send request\nmessage: {}", res.message)
            } else {
                println!("failed to send request\nmessage: {}", res.message)
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failes\nError: {}", e.message());
        }
    }
    Ok(())
}

pub async fn daemon_list(
    client: &mut ContainerServiceClient<Channel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request = Request::new(ListRequest {});

    match client.list(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.table.trim().is_empty() {
                println!("No container found.")
            } else {
                print!("{}", res.table)
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failed\nError: {}", e.message());
        }
    }

    Ok(())
}

// run request
pub async fn daemon_run(
    client: &mut ContainerServiceClient<Channel>,
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request = Request::new(RunRequest {
        container_id: container_id.to_string(),
    });

    match client.run(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("success to send request\nmessage: {}", res.message)
            } else {
                println!("failed to send request\nmessage: {}", res.message)
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failes\nError: {}", e.message());
        }
    }
    Ok(())
}

// kill request
pub async fn daemon_kill(
    client: &mut ContainerServiceClient<Channel>,
    container_id: &str,
    signal: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request = Request::new(KillRequest {
        container_id: container_id.to_string(),
        signal: signal.to_string(),
    });

    match client.kill(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("success to send request\nmessage: {}", res.message)
            } else {
                println!("failed to send request\nmessage: {}", res.message)
            }
        }
        Err(e) => {
            eprintln!("GRPC Connection failes\nError: {}", e.message());
        }
    }
    Ok(())
}
