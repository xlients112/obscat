// use crate::grpcutils;
use crate::xserve_proto::container_service_client::ContainerServiceClient;
use crate::xserve_proto::{
    CreateRequest, DeleteRequest, KillRequest, NetworkConfig, RunRequest, StateRequest,
};
use tonic::{Request, transport::Channel};

// create request
pub async fn daemon_create(
    client: &mut ContainerServiceClient<Channel>,
    container_id: &str,
    image_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting build container...");
    let net_config = NetworkConfig {
        mode: "bridge".to_string(),
        ip_addres: "".to_string(),
        port: "".to_string(),
    };
    let request = Request::new(CreateRequest {
        container_id: container_id.to_string(),
        image_id: image_id.to_string(),
        // bundle: bundle.to_string(),
        network: Some(net_config),
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
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting to delete container...");

    let request = Request::new(DeleteRequest {
        container_id: container_id.to_string(),
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

