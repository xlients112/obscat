use std::{error::Error, fs, path::PathBuf};

use clap::{Parser, Subcommand};
use libcontainer::{
    container::{Container, builder::ContainerBuilder},
    signal::Signal,
    syscall::syscall::SyscallType,
    workload::default::DefaultExecutor,
};
use liboci_cli::{CommonCmd, Exec, GlobalOpts, List, Spec, StandardCmd};
use tokio::process::Command;

use crate::utils::YOUKI_ROOT;

#[derive(Parser, Debug)]
pub struct ContainerCli {
    #[clap(flatten)]
    pub global: GlobalOpts,

    #[clap(subcommand)]
    pub cmd: WorkerSubCmd,
}

#[derive(Subcommand, Debug)]
pub enum WorkerSubCmd {
    #[clap(flatten)]
    Standard(StandardCmd),

    #[clap(flatten)]
    Common(CommonCmd),
}
// calling coomand
pub fn container_cmd() -> Result<Command, Box<dyn Error + Send + Sync>> {
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("__worker");
    Ok(cmd)
}

pub fn run_worker_container(
    cli: ContainerCli,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root_path = cli.global.root.unwrap_or_else(|| PathBuf::from(YOUKI_ROOT));

    match cli.cmd {
        WorkerSubCmd::Standard(cmd) => match cmd {
            // container create
            StandardCmd::Create(args) => {
                ContainerBuilder::new(args.container_id.clone(), SyscallType::default())
                    .with_executor(DefaultExecutor {})
                    .with_pid_file(args.pid_file.as_ref())?
                    .with_console_socket(args.console_socket.as_ref())
                    .with_root_path(root_path)?
                    .as_init(args.bundle)
                    .with_systemd(false)
                    .build()?;
            }

            // container delete
            StandardCmd::Delete(args) => {
                let mut container = Container::load(root_path.join(&args.container_id))?;
                container.delete(args.force)?;
            }

            // container start
            StandardCmd::Start(args) => {
                let mut container = Container::load(root_path.join(&args.container_id))?;
                if container.status() == libcontainer::container::ContainerStatus::Running {
                    return Ok(());
                }
                container.start()?;
            }

            // Container Kill
            StandardCmd::Kill(args) => {
                let mut container = Container::load(root_path.join(&args.container_id))?;
                let signal: Signal = args.signal.as_str().try_into()?;
                container.kill(signal, args.all)?;
            }

            //  container state
            StandardCmd::State(args) => {
                let contianer = Container::load(root_path.join(&args.container_id))?;
                println!("{}", serde_json::to_string_pretty(&contianer.state)?);
            }
        },

        WorkerSubCmd::Common(cmd) => match cmd {
            CommonCmd::List(args) => handle_list(args, root_path),
            CommonCmd::Spec(args) => handle_spec(args),
            CommonCmd::Exec(args) => handle_exec(args, root_path),
            _ => return Err("command not found".into()),
        }?,
    }

    Ok(())
}

fn handle_list(_args: List, root_path: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut state = Vec::new();

    if root_path.exists() {
        for entry in fs::read_dir(&root_path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                if let Ok(container) = Container::load(entry.path()) {
                    state.push(container.state);
                }
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&state)?);
    Ok(())
}

fn handle_exec(args: Exec, root_path: PathBuf) -> Result<(), Box<dyn Error + Send + Sync>> {
    ContainerBuilder::new(args.container_id.clone(), SyscallType::default())
        .with_executor(DefaultExecutor {})
        .with_root_path(root_path)?
        .with_console_socket(args.console_socket.as_ref())
        .with_pid_file(args.pid_file.as_ref())?
        .as_tenant()
        .with_cwd(args.cwd.as_ref())
        .with_env(args.env.into_iter().collect())
        .with_process(args.process.as_ref())
        .with_no_new_privs(args.no_new_privs)
        .with_container_args(args.command.clone());

    //     if !args.detach {
    //     match waitpid(pid, None)? {
    //         WaitStatus::Exited(_, code) if code != 0 => std::process::exit(code),
    //         WaitStatus::Signaled(_, sig, _) => std::process::exit(128 + sig as i32),
    //         _ => {}
    //     }
    // }

    Ok(())
}

fn handle_spec(args: Spec) -> Result<(), Box<dyn Error + Send + Sync>> {
    // let spec = oci_spec::runtime::Spec::default();
    // let out_dir = args.bundle.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // spec.save(out_dir.join("config.json"));

    let out_dir = args
        .bundle
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    oci_spec::runtime::Spec::default().save(out_dir.join("config.json"))?;
    Ok(())
}
