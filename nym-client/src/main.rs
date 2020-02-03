use clap::{App, ArgMatches};

pub mod built_info;
pub mod client;
mod commands;
pub mod config;
mod sockets;

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let arg_matches = App::new("Nym Client")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of the Nym Client")
        .subcommand(commands::init::command_args())
        .subcommand(commands::run::command_args())
        .get_matches();

    execute(arg_matches);
}

fn execute(matches: ArgMatches) {
    match matches.subcommand() {
        ("init", Some(m)) => {
            println!("{}", banner());
            commands::init::execute(m);
        }
        ("run", Some(m)) => {
            println!("{}", banner());
            commands::run::execute(m);
        }
        _ => {
            println!("{}", usage());
        }
    }
}

fn usage() -> String {
    banner() + "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (client - version {:})

    "#,
        built_info::PKG_VERSION
    )
}
