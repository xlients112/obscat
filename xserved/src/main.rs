pub mod internal_proto {
    tonic::include_proto!("xserve");
}
pub mod runtime {
    pub mod init;
    pub mod internal;
    pub mod logging;
    pub mod network;
    pub mod utils;
    pub mod stream;
}
pub mod grpcutils;

use crate::internal_proto::{
    CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, KillRequest, KillResponse,
    RunRequest, RunResponse, StateRequest, StateResponse,
};
use crate::runtime::logging::LogsCollector;
use crate::runtime::{init, internal, network};
use internal_proto::container_service_server::{ContainerService, ContainerServiceServer};
use tonic::{Request, Response, Status, transport::Server};

#[derive(Debug, Default)]
pub struct XserveService {}
#[tonic::async_trait]
impl ContainerService for XserveService {
    // create container
    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let image = req.image_id.clone();
        let net = req.network.as_ref().cloned().unwrap_or_default();
        let mut log = LogsCollector::new();

        let log = tokio::spawn(async move {
            if let Err(e) = internal::runtime_create(&cid, &image, &net, &mut log).await {
                log.info(format!("failed to run runtime create task\nError: {}", e));
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(CreateResponse {
            sucess: true,
            message: log.flush(),
        })
    }

    // delete
    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let mut log = LogsCollector::new();

        let log = tokio::spawn(async move {
            if let Err(e) = internal::runtime_delete(&cid, &mut log).await {
                eprintln!("Failed to run task delete container\nError: {}", e);
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(DeleteResponse {
            sucess: true,
            message: log.flush(),
        })
    }

    // run container
    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let mut log = LogsCollector::new();
        // tokio spawn run
        let log = tokio::spawn(async move {
            if let Err(e) = internal::container_run(&cid, &mut log).await {
                eprintln!("Gagal menjalankan task kontainer: {}", e);
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(RunResponse {
            sucess: true,
            message: log.flush(),
        })
    }

    // Kill container
    async fn kill(&self, request: Request<KillRequest>) -> Result<Response<KillResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let sig = req.signal.clone();
        let mut log = LogsCollector::new();
        let log = tokio::spawn(async move {
            if let Err(e) = internal::runtime_kill(&cid, &sig, &mut log).await {
                eprintln!("Failed to kill container\nError: {}", e)
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(KillResponse {
            sucess: true,
            message: log.flush(),
        })
    }

    // state container
    async fn state(
        &self,
        request: Request<StateRequest>,
    ) -> Result<Response<StateResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let mut log = LogsCollector::new();
        // tokio spawn state
        let log = tokio::spawn(async move {
            if let Err(e) = internal::state(&cid, &mut log).await {
                eprintln!("Failed to run state task\n Error: {}", e);
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(StateResponse {
            sucess: true,
            message: log.flush(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // init network
    if let Err(e) = network::init_global_bridge().await {
        eprintln!("Failed to init network global\nError: {}", e)
    }

    // init global system
    if let Err(e) = init::init_system().await {
        eprintln!("Failed to init global system\nError: {}", e)
    }

    // grpc server
    let addr = "0.0.0.0:5005".parse()?;
    let xserve_service = XserveService::default();
    println!("grpc start {}", addr);
    Server::builder()
        .add_service(ContainerServiceServer::new(xserve_service))
        .serve(addr)
        .await?;
    println!("daemon server has been started at {}", addr);

    Ok(())
}
