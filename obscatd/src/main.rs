pub mod internal_proto {
    tonic::include_proto!("obscat");
}

pub mod container;
pub mod deployment;
pub mod supervisor;
pub mod grpcutils;
pub mod runtime;
pub mod utils;
// pub mod mod;

use crate::container::create::{container_create, container_delete};
use crate::deployment::deployment::{deploy, undeploy};
use crate::internal_proto::{
    CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, DeployRequest, DeployResponse, KillRequest, KillResponse, ListRequest, ListResponse, RestartRequest, RestartResponse, RunRequest, RunResponse, StateRequest, StateResponse, StreamLogsRequest, StreamLogsResponse, UndeployRequest, UndeployResponse,
};
use crate::runtime::container_internal::{ContainerCli, run_worker_container};
use crate::runtime::logging::{init_global_capture, subscribe_logs};
use crate::runtime::{db, init, internal, network};
use crate::supervisor::restart::restart;
use clap::{Parser, Subcommand};
use internal_proto::container_service_server::{ContainerService, ContainerServiceServer};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status, transport::Server};

type LogStream = std::pin::Pin<
    Box<dyn tokio_stream::Stream<Item = Result<StreamLogsResponse, Status>> + Send + 'static>,
>;

#[derive(Debug, Default)]
pub struct ObscatService {}

// worker
#[derive(Parser, Debug)]
struct AppCli {
    #[clap(subcommand)]
    command: Option<AppSubcommand>,
}
#[derive(Subcommand, Debug)]
enum AppSubcommand {
    #[clap(name = "__worker", hide = true)]
    Worker(ContainerCli),
}

#[tonic::async_trait]
impl ContainerService for ObscatService {
    type StreamLogsStream = LogStream;

    async fn stream_logs(
        &self,
        _request: Request<StreamLogsRequest>,
    ) -> Result<Response<Self::StreamLogsStream>, Status> {
        let rx = subscribe_logs();

        let stream = BroadcastStream::new(rx).filter_map(|res| match res {
            Ok(line) => Some(Ok(StreamLogsResponse { content: line })),
            Err(_) => None,
        });

        Ok(Response::new(Box::pin(stream)))
    }

    // deploy
    async fn deploy(
        &self,
        request: Request<DeployRequest>,
    ) -> Result<Response<DeployResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.package_path.clone();
        // let net = req.network.as_ref().cloned().unwrap_or_default();

        tokio::spawn(async move {
            if let Err(e) = deploy(&package_path).await {
                println!("failed to run runtime deploy task\nError: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(DeployResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // undeploy
    async fn undeploy(
        &self,
        request: Request<UndeployRequest>,
    ) -> Result<Response<UndeployResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.package_path.clone();

        tokio::spawn(async move {
            if let Err(e) = undeploy(&package_path).await {
                println!("failed to run runtime undeploy task\nError: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(UndeployResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // test create

    // create container
    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.container_id.clone();

        tokio::spawn(async move {
            if let Err(e) = container_create(&package_path).await {
                println!("failed to run runtime create task\nError: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(CreateResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // delete
    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = grpcutils::unpack(request);
        let package_path = req.container_id.clone();

        tokio::spawn(async move {
            if let Err(e) = container_delete(&package_path).await {
                eprintln!("Failed to run task delete container\nError: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(DeleteResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // restart
        async fn restart(
        &self,
        request: Request<RestartRequest>,
    ) -> Result<Response<RestartResponse>, Status> {
        let req = grpcutils::unpack(request);
        let container_id = req.container_id.clone();

        tokio::spawn(async move {
            if let Err(e) = restart(&container_id).await {
                eprintln!("Failed to run task restart container\nError: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(RestartResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // run container
    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        // tokio spawn run
        tokio::spawn(async move {
            if let Err(e) = internal::container_start(&cid).await {
                eprintln!("Gagal menjalankan task kontainer: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(RunResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // Kill container
    async fn kill(&self, request: Request<KillRequest>) -> Result<Response<KillResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        let sig = req.signal.clone();

        tokio::spawn(async move {
            if let Err(e) = internal::runtime_kill(&cid, &sig).await {
                eprintln!("Failed to kill container\nError: {}", e)
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(KillResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    // state container
    async fn state(
        &self,
        request: Request<StateRequest>,
    ) -> Result<Response<StateResponse>, Status> {
        let req = grpcutils::unpack(request);
        let cid = req.container_id.clone();
        // tokio spawn state
        tokio::spawn(async move {
            if let Err(e) = internal::state(&cid).await {
                eprintln!("Failed to run state task\n Error: {}", e);
            }
        })
        .await
        .unwrap_or_default();

        grpcutils::pack(StateResponse {
            sucess: true,
            message: "OK".to_string(),
        })
    }

    async fn list(&self, _request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let table = match internal::runtime_list().await {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Failed to run task list, Error: {}", e);
                String::new()
            }
        };
        grpcutils::pack(ListResponse { table })
    }
}


fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = AppCli::parse();

    // calling singgle thread worker
    if let Some(AppSubcommand::Worker(worker_cli)) = cli.command {
        return run_worker_container(worker_cli);
    }

    // return tokio multi thread
    tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?
    .block_on(tokio_main())
}


async fn tokio_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // debug capture
    let _ = init_global_capture();
    // init network
    if let Err(e) = network::init_global_bridge().await {
        eprintln!("Failed to init network global\nError: {}", e)
    }

    // init global system
    if let Err(e) = init::init_system().await {
        eprintln!("Failed to init global system\nError: {}", e)
    }

    // init database
    db::con::init_db("/var/lib/obscatd/state.db")
        .await
        .expect("Failed to initialized database SQLite");

    // grpc server
    let addr = "0.0.0.0:5005".parse()?;
    let obscat_service = ObscatService::default();
    println!("grpc start {}", addr);
    Server::builder()
        .add_service(ContainerServiceServer::new(obscat_service))
        .serve(addr)
        .await?;
    println!("obscatd daemon server has been started at {}", addr);

    Ok(())
}
