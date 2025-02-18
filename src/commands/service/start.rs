use clap::{ArgMatches, Args, Command, FromArgMatches, ValueEnum};

use common::{errors::MegaResult, model::CommonOptions};
use gateway::{
    https_server::{self, HttpCustom, HttpOptions},
    ssh_server::{self, SshCustom, SshOptions},
};
use p2p::peer::{self, P2pCustom, P2pOptions};

use crate::cli::Config;

#[derive(Debug, PartialEq, Clone, ValueEnum)]
enum ServiceType {
    Http,
    Https,
    Ssh,
    P2p,
}

#[derive(Args, Clone, Debug)]
pub struct StartOptions {
    service: Vec<ServiceType>,

    #[clap(flatten)]
    pub common: CommonOptions,

    #[clap(flatten)]
    pub http: HttpCustom,

    #[clap(flatten)]
    pub ssh: SshCustom,

    #[clap(flatten)]
    pub p2p: P2pCustom,
}

pub fn cli() -> Command {
    StartOptions::augment_args_for_update(
        Command::new("start").about("Start multiple server by given params"),
    )
}

pub(crate) async fn exec(_config: Config, args: &ArgMatches) -> MegaResult {
    let server_matchers = StartOptions::from_arg_matches(args)
        .map_err(|err| err.exit())
        .unwrap();

    println!("{server_matchers:#?}");

    let service_type = server_matchers.service;

    let http_server = if service_type.contains(&ServiceType::Http)
        || service_type.contains(&ServiceType::Https)
    {
        let http = HttpOptions {
            common: server_matchers.common.clone(),
            custom: server_matchers.http,
        };
        tokio::spawn(async move { https_server::start_server(&http).await })
    } else {
        tokio::task::spawn(async {})
    };

    let ssh_server = if service_type.contains(&ServiceType::Ssh) {
        let ssh = SshOptions {
            common: server_matchers.common.clone(),
            custom: server_matchers.ssh,
        };
        tokio::spawn(async move { ssh_server::start_server(&ssh).await })
    } else {
        tokio::task::spawn(async {})
    };

    let p2p_server = if service_type.contains(&ServiceType::P2p) {
        let p2p = P2pOptions {
            common: server_matchers.common.clone(),
            custom: server_matchers.p2p,
        };
        tokio::spawn(async move {
            peer::run(&p2p).await.unwrap();
        })
    } else {
        tokio::task::spawn(async {})
    };

    let _ = tokio::join!(http_server, ssh_server, p2p_server);

    Ok(())
}
