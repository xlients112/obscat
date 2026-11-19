pub mod internal_proto {
    tonic::include_proto!("xserve");
}

pub mod deployment;
pub mod grpcutils;
pub mod runtime;
// pub mod mod;

use crate::deployment::deployment::{deploy, undeploy};
use crate::internal_proto::{
    CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, DeployRequest, DeployResponse, KillRequest, KillResponse, RunRequest, RunResponse, StateRequest, StateResponse, UndeployRequest, UndeployResponse,
};
use crate::runtime::logging::LogsCollector;
use crate::runtime::{init, internal, network};
use internal_proto::container_service_server::{ContainerService, ContainerServiceServer};
use tonic::{Request, Response, Status, transport::Server};

#[derive(Debug, Default)]
pub struct XserveService {}
#[tonic::async_trait]
impl ContainerService for XserveService {
    // deploy
    async fn deploy(
        &self,
        request: Request<DeployRequest>,
    ) -> Result<Response<DeployResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.package_path.clone();
        // let net = req.network.as_ref().cloned().unwrap_or_default();
        let mut log = LogsCollector::new();

        let log = tokio::spawn(async move {
            if let Err(e) = deploy(&package_path, &mut log).await {
                log.info(format!("failed to run runtime deploy task\nError: {}", e));
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(DeployResponse {
            sucess: true,
            message: log.flush(),
        })
    }

       // undeploy
    async fn undeploy(
        &self,
        request: Request<UndeployRequest>,
    ) -> Result<Response<UndeployResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.package_path.clone();
        // let net = req.network.as_ref().cloned().unwrap_or_default();
        let mut log = LogsCollector::new();

        let log = tokio::spawn(async move {
            if let Err(e) = undeploy(&package_path, &mut log).await {
                log.info(format!("failed to run runtime undeploy task\nError: {}", e));
            }
            log
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(UndeployResponse {
            sucess: true,
            message: log.flush(),
        })
    }

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
