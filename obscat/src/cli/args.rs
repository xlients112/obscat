use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "obscat")]
#[command(author, version, about = "obscat container engine manager", long_about = None)]
pub struct Cli {
    // #[ some_command ]
    #[command(subcommand)]
    pub command: Commands,
}
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum VendorList {
    DockerHub,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // basic cmd
    Version,
    Init {
        folder: String, 
    },

    // daemon start, check, stop
    DaemonStart {

    },
    DaemonStop {

    },
    DaemonCheck {},

    // image manager
    Image(ImageArgs),

    // deploy cli
    Deploy {
        path: String,
    },
    Undeploy {
        path: String,
    },

    // CLI COMMANDS INTI KE DAEMON
    Create {
        id: String,
        image: String,
    },
    Delete {
        id: String,
    },
    State {
        id: String,
    },
    Run {
        id: String,
    },
    Kill {
        id: String,
        signal: String,
    },
}

#[derive(Args, Debug)]
pub struct ImageArgs {
    #[command(subcommand)]
    pub action: ImageSubcommands,
}
#[derive(Subcommand, Debug)]
pub enum ImageSubcommands {
    Pull {
        vendor: VendorList,
        image: String,
    },
    List {},

    Remove {
        image_spec: String,
    },
}
