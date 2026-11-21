pub mod cli {
    pub mod args;
    pub mod cummon;
}
pub mod daemon {
    pub mod deploy;
    pub mod image;
    pub mod internal;
}
pub mod obscat_proto {
    tonic::include_proto!("obscat");
}
pub mod grpcutils;

use std::fs;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tonic;

use crate::cli::args::{Cli, Commands, ImageSubcommands};
use crate::cli::cummon;
use crate::daemon::deploy::{daemon_deploy, daemon_undeploy};
use crate::daemon::image;
use crate::daemon::internal;
use crate::obscat_proto::StreamLogsRequest;
use crate::obscat_proto::container_service_client::ContainerServiceClient;
use clap::Parser;
use tonic::transport::Channel;

const PID_FILE: &str = "/var/run/obscatd/obscatd.pid";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // CLAPS CLI PARSER
    let args = Cli::parse();

    let is_command_daemon = matches!(
        args.command,
        Commands::DaemonStart {} | Commands::DaemonStop {} | Commands::DaemonCheck {}
    );

    let mut client = if is_command_daemon {
        None
    } else {
        // grpc connection
        let endpoint =
            Channel::from_static("http://127.0.0.1:5005").connect_timeout(Duration::from_secs(3));

        let channel = endpoint.connect().await.map_err(|e| {
        format!(
            "Failed to connect obscat daemon\n\nensure the obscat daemon is started.\nError: {}",
            e
        )
    })?;
        let grpc_client = ContainerServiceClient::new(channel);

        let mut log_client = grpc_client.clone();
        tokio::spawn(async move {
            if let Ok(resp) = log_client.stream_logs(StreamLogsRequest {}).await {
                let mut stream = resp.into_inner();
                while let Ok(Some(msg)) = stream.message().await {
                    println!("{}", msg.content);
                }
            }
        });
        Some(grpc_client)
    };

    match args.command {
        // basic command
        Commands::Version => {
            if let Err(e) = cummon::cummon_version().await {
                eprintln!("failed to run task command version\nError: {}", e);
            }
        }
        Commands::InitDeployment { folder } => {
            if let Err(e) = cummon::init_deployment(&folder).await {
                eprintln!("failed to run task command init\nError: {}", e);
            }
        }
        Commands::InitManifest {  } => {
            if let Err(e) = cummon::init_manifest().await {
                eprintln!("failed to run task command init\nError: {}", e)
            }
        }

        // daemon command
        Commands::DaemonStart {} => {
            if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
                let pid = pid_str.trim();
                if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                    println!("Daemon obscat is already running (PID: {})", pid);
                    return Ok(());
                }
            }

            if let Err(e) = Command::new("obscatd")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                eprintln!("Failed to starting daemon\nError: {}", e);
                return Ok(());
            }
            for _ in 0..30 {
                if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
                    let pid = pid_str.trim();
                    if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                        println!("Daemon obscatd sucess to run at (PID: {})", pid);
                        return Ok(());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            println!("Daemon procces has been started.");
            return Ok(());
        }
        Commands::DaemonStop {} => {
            match fs::read_to_string(PID_FILE) {
                Ok(pid_str) => {
                    let pid = pid_str.trim();
                    // Kirim sinyal SIGTERM ke PID tersebut
                    match Command::new("kill").args(["-15", pid]).status().await {
                        Ok(status) if status.success() => {
                            let _ = fs::remove_file(PID_FILE);
                            println!("Daemon obscatd (PID: {}) has been killed.", pid);
                        }
                        Ok(_) => {
                            // Proses mungkin sudah mati sebelumnya
                            let _ = fs::remove_file(PID_FILE);
                            eprintln!("PID process {} not found, deleting PID file...", pid);
                        }
                        Err(e) => eprintln!("Failed to killed obscatd daemon\nError: {}", e),
                    }
                }
                Err(_) => {
                    eprintln!(
                        "Daemon obscatd is not running (PID file not found)\nTry: obscat daemon-start"
                    );
                }
            }
        }
        Commands::DaemonCheck {} => match fs::read_to_string(PID_FILE) {
            Ok(pid_str) => {
                let pid = pid_str.trim();
                if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                    println!("Status: obscatd is ACTIVE (PID: {})", pid);
                } else {
                    println!(
                        "Status: obscatd DIED/Not running (the old PID file found for PID: {}, cleaning...)",
                        pid
                    );
                    let _ = fs::remove_file(PID_FILE);
                }
            }
            Err(_) => {
                println!("Status: obscatd is not RUNNING");
            }
        },

        // image command
        Commands::Image(image_args) => match image_args.action {
            ImageSubcommands::Pull { vendor, image } => {
                if let Err(e) = image::image_pull(&vendor, &image).await {
                    eprintln!("failed to run task command image pull\nError: {}", e);
                }
            }

            ImageSubcommands::List {} => match image::image_list().await {
                Ok(images) => {
                    if images.is_empty() {
                        println!("no images found.\nTry obscat pull to download image");
                    } else {
                        println!("{:<30} {:<15} {:<15}", "REPOSITORY", "TAG", "SIZE(byte)");
                        for (name, tag, size) in images {
                            println!("{:<30} {:<15} {:<15}", name, tag, size);
                        }
                    }
                }

                Err(e) => {
                    eprintln!("Failed to run task image list\nError: {}", e);
                }
            },

            ImageSubcommands::Remove { image_spec } => {
                if let Err(e) = image::image_remove(&image_spec).await {
                    eprintln!("failed to run task command image remove\nError: {}", e);
                }
            }
        },

        // deploy command
        Commands::Deploy { path } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = daemon_deploy(client, &path).await {
                eprintln!("Failed to run task command deploy\nError: {}", e);
            }
        }
        Commands::Undeploy { path } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = daemon_undeploy(client, &path).await {
                eprintln!("Failed to run task command undeploy\nError: {}", e);
            }
        }

        Commands::Create { path } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_create(client, &path).await {
                eprintln!("Failed to run task command create\nError: {}", e);
            }
        }

        Commands::Delete { id } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_delete(client, &id).await {
                eprintln!("failed to run task command delete\nError: {}", e)
            }
        }

        Commands::State { id } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_state(client, &id).await {
                eprintln!("failed to run task command state\nError: {}", e)
            }
        }

        Commands::List {  } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_list(client).await {
                eprintln!("failed to run task command list\nError: {}", e)
            }
        }

        Commands::Run { id } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_run(client, &id).await {
                eprintln!("failed to run task command state\nError: {}", e)
            }
        }

        Commands::Kill { id, signal } => {
            let client = client.as_mut().unwrap();
            if let Err(e) = internal::daemon_kill(client, &id, &signal).await {
                eprintln!("failed to run task command state\nError: {}", e)
            }
        }
    }
    Ok(())
}
