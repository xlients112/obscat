use std::path::Path;

use tonic::Request;
use tonic::transport::Channel;

use crate::xserve_proto::{
    DeployRequest, UndeployRequest, container_service_client::ContainerServiceClient,
};

pub async fn daemon_deploy(
    client: &mut ContainerServiceClient<Channel>,
    package_path: &String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting deploy container...");

    let raw_path = Path::new(package_path);
    let abs_path = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };
    if !abs_path.exists() {
        return Err(format!("Deployment folder is not found: {:?}", abs_path).into());
    }
    let final_package_path = abs_path.to_string_lossy().to_string();

    let request = Request::new(DeployRequest {
        package_path: final_package_path,
    });

    match client.deploy(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("Deploy request success\nmessage: {}", res.message);
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


pub async fn daemon_undeploy(
    client: &mut ContainerServiceClient<Channel>,
    package_path: &String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting un-deploy container...");

    let raw_path = Path::new(package_path);
    let abs_path = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };
    if !abs_path.exists() {
        return Err(format!("Deployment folder is not found: {:?}", abs_path).into());
    }
    let final_package_path = abs_path.to_string_lossy().to_string();

    let request = Request::new(UndeployRequest {
        package_path: final_package_path,
    });

    match client.undeploy(request).await {
        Ok(response) => {
            let res = response.into_inner();
            if res.sucess {
                println!("Deploy request success\nmessage: {}", res.message);
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