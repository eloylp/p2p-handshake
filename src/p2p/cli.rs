use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version)]
#[command(propagate_version = true)]
pub struct HandshakeConfig {
    #[arg(
        long,
        short,
        default_value_t = 500,
        help = "maximum per handshake operation time in ms"
    )]
    pub timeout: u64,
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Btc {
        nodes_addrs: Vec<String>,
        #[arg(
            long,
            short,
            help = "the user agent to be used during handshake operation",
            default_value = "/Satoshi:23.0.0/"
        )]
        user_agent: String,
    },
}
