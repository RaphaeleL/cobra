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
                .arg(
                    Arg::new("delete")
                        .short('d')
                        .long("delete")
                        .help("Delete a branch")
                        .value_name("NAME")
                        .num_args(1)
                )
                .arg(
                    Arg::new("merge")
                        .short('m')
                        .long("merge")
                        .help("Merge a branch into the current branch")
                        .value_name("NAME")
                        .num_args(1)
                )
        )
        .subcommand(
            Command::new("checkout")
                .about("Switch branches or restore files")
                .arg(
                    Arg::new("path")
                        .help("Branch name or file path to checkout")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("rebase")
                .about("Reapply commits on top of another base tip")
                .arg(
                    Arg::new("branch")
                        .help("Branch to rebase onto")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("stash")
                .about("Stash changes in a dirty working directory")
                .subcommand(
                    Command::new("push")
                        .about("Save your local modifications to a new stash")
                        .arg(
                            Arg::new("message")
                                .help("Optional message for the stash")
                                .short('m')
                                .long("message")
                        )
                )
                .subcommand(
                    Command::new("list")
                        .about("List all stashes")
                )
                .subcommand(
                    Command::new("show")
                        .about("Show the contents of a stash")
                        .arg(
                            Arg::new("stash")
                                .help("Stash reference (e.g., stash@{0})")
                                .default_value("stash@{0}")
                        )
                )
                .subcommand(
                    Command::new("apply")
                        .about("Apply a stash to the working directory")
                        .arg(
                            Arg::new("stash")
                                .help("Stash reference (e.g., stash@{0})")
                                .default_value("stash@{0}")
                        )
                )
                .subcommand(
                    Command::new("drop")
                        .about("Remove a stash from the stash list")
                        .arg(
                            Arg::new("stash")
                                .help("Stash reference (e.g., stash@{0})")
                                .default_value("stash@{0}")
                        )
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
            } else if let Some(name) = sub_matches.get_one::<String>("delete") {
                commands::branch::delete(name)
            } else if let Some(name) = sub_matches.get_one::<String>("merge") {
                commands::branch::merge(name)
            } else {
                println!("No branch subcommand was used");
                Ok(())
            }
        },
        Some(("checkout", sub_matches)) => {
            let path = sub_matches.get_one::<String>("path").unwrap();
            commands::checkout::run(path)
        },
        Some(("rebase", sub_matches)) => {
            let branch = sub_matches.get_one::<String>("branch").unwrap();
            commands::rebase::run(branch)
        },
        Some(("stash", sub_matches)) => {
            match sub_matches.subcommand() {
                Some(("push", sub_matches)) => {
                    let message = sub_matches.get_one::<String>("message");
                    commands::stash::push(message)
                },
                Some(("list", _)) => {
                    commands::stash::list()
                },
                Some(("show", sub_matches)) => {
                    let stash = sub_matches.get_one::<String>("stash").unwrap();
                    commands::stash::show(stash)
                },
                Some(("apply", sub_matches)) => {
                    let stash = sub_matches.get_one::<String>("stash").unwrap();
                    commands::stash::apply(stash)
                },
                Some(("drop", sub_matches)) => {
                    let stash = sub_matches.get_one::<String>("stash").unwrap();
                    commands::stash::drop(stash)
                },
                _ => {
                    println!("No stash subcommand was used");
                    Ok(())
                }
            }
        },
        _ => {
            println!("No subcommand was used");
            Ok(())
        }
    }
} 