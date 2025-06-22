// CLI parsing and command routing 

use clap::{Command, Arg};
use std::io;
use crate::cobra::commands;

pub fn run() -> io::Result<()> {
    let matches = Command::new("cobra")
        .version("0.1.0")
        .about("A Git implementation in Rust")
        .subcommand(
            Command::new("init")
                .about("Create an empty Cobra repository")
                .arg(
                    Arg::new("path")
                        .help("Where to create the repository")
                        .default_value(".")
                )
        )
        .subcommand(
            Command::new("add")
                .about("Add file contents to the index")
                .arg(
                    Arg::new("path")
                        .help("Files to add")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("commit")
                .about("Record changes to the repository")
                .arg(
                    Arg::new("message")
                        .short('m')
                        .long("message")
                        .help("The commit message")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("log")
                .about("Show commit logs")
        )
        .subcommand(
            Command::new("status")
                .about("Show the working tree status")
        )
        .subcommand(
            Command::new("branch")
                .about("Branch operations")
                .arg(
                    Arg::new("create")
                        .short('c')
                        .long("create")
                        .help("Create a new branch")
                        .value_name("NAME")
                        .num_args(1)
                )
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("List all branches")
                        .action(clap::ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("switch")
                        .short('s')
                        .long("switch")
                        .help("Switch to a branch")
                        .value_name("NAME")
                        .num_args(1)
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("init", sub_matches)) => {
            let path = sub_matches.get_one::<String>("path").unwrap();
            commands::init::run(path)
        },
        Some(("add", sub_matches)) => {
            let path = sub_matches.get_one::<String>("path").unwrap();
            commands::add::run(path)
        },
        Some(("commit", sub_matches)) => {
            let message = sub_matches.get_one::<String>("message").unwrap();
            commands::commit::run(message)
        },
        Some(("log", _)) => {
            commands::log::run()
        },
        Some(("status", _)) => {
            commands::status::run()
        },
        Some(("branch", sub_matches)) => {
            if let Some(name) = sub_matches.get_one::<String>("create") {
                commands::branch::create(name)
            } else if sub_matches.get_flag("all") {
                commands::branch::list()
            } else if let Some(name) = sub_matches.get_one::<String>("switch") {
                commands::branch::switch(name)
            } else {
                println!("No branch subcommand was used");
                Ok(())
            }
        },
        _ => {
            println!("No subcommand was used");
            Ok(())
        }
    }
} 