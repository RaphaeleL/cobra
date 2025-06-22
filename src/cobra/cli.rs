// CLI parsing and command routing 

use clap::{Command, Arg};
use std::io;
use crate::cobra::commands;

pub fn run() -> io::Result<()> {
    let matches = Command::new("cobra")
        .version("1.0")
        .about("A Git-like version control system")
        .subcommand(
            Command::new("init")
                .about("Initialize a new repository")
                .arg(
                    Arg::new("path")
                        .help("Path to initialize repository in")
                        .default_value(".")
                )
        )
        .subcommand(
            Command::new("add")
                .about("Add file contents to the index")
                .arg(
                    Arg::new("file")
                        .help("File to add")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("commit")
                .about("Record changes to the repository")
                .arg(
                    Arg::new("message")
                        .help("Commit message")
                        .short('m')
                        .long("message")
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
                .about("List, create, or delete branches")
                .subcommand(
                    Command::new("list")
                        .about("List all branches")
                        .alias("ls")
                )
                .subcommand(
                    Command::new("create")
                        .about("Create a new branch")
                        .arg(
                            Arg::new("name")
                                .help("Name of the branch to create")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("checkout")
                        .about("Switch to a branch")
                        .arg(
                            Arg::new("name")
                                .help("Name of the branch to switch to")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a branch")
                        .arg(
                            Arg::new("name")
                                .help("Name of the branch to delete")
                                .required(true)
                        )
                )
                .subcommand(
                    Command::new("merge")
                        .about("Merge a branch into the current branch")
                        .arg(
                            Arg::new("name")
                                .help("Name of the branch to merge")
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
            let file = sub_matches.get_one::<String>("file").unwrap();
            commands::add::run(file)
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
            match sub_matches.subcommand() {
                Some(("list", _)) => {
                    commands::branch::list()
                },
                Some(("create", sub_matches)) => {
                    let name = sub_matches.get_one::<String>("name").unwrap();
                    commands::branch::create(name)
                },
                Some(("checkout", sub_matches)) => {
                    let name = sub_matches.get_one::<String>("name").unwrap();
                    commands::branch::switch(name)
                },
                Some(("delete", sub_matches)) => {
                    let name = sub_matches.get_one::<String>("name").unwrap();
                    commands::branch::delete(name)
                },
                Some(("merge", sub_matches)) => {
                    let name = sub_matches.get_one::<String>("name").unwrap();
                    commands::branch::merge(name)
                },
                Some(("rebase", sub_matches)) => {
                    let branch = sub_matches.get_one::<String>("branch").unwrap();
                    commands::branch::rebase(branch)
                },
                _ => {
                    // Default to list if no subcommand specified
                    commands::branch::list()
                }
            }
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