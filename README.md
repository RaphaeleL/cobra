# Cobra

Cobra is a Git-like version control system implemented in Rust. It provides basic version control functionality similar to Git, allowing you to track changes in your codebase, create commits, and manage your project's history.

## Installation

1. Make sure you have Rust and Cargo installed
2. Clone this repository
3. Build the project: `cargo build --release`
4. The binary will be available at `target/release/cobra`

## Commands

```
$ cobra -h
A Git-like version control system

Usage: cobra [COMMAND]

Commands:
  init    Initialize a new repository
  add     Add file contents to the index
  commit  Record changes to the repository
  log     Show commit logs
  status  Show the working tree status
  branch  List, create, or delete branches
  stash   Stash changes in a dirty working directory
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Implementation Details

Cobra follows Git's internal object model:

- **Blob Objects**: Store file contents
- **Tree Objects**: Represent directories and file hierarchies
- **Commit Objects**: Store commit metadata and point to trees
- **References**: Track branches and HEAD position

The repository structure is similar to Git:
```
.cobra/
  ├── HEAD
  ├── index
  ├── objects/
  └── refs/
      └── heads/
```

> HINT: Since there is no Remote Repository, the `log` command is showing commited changes, not pushed changes. Also, `status` is handling commited changes, like pushed changes. In addition to that reason, there is no `push` or `remote` command. Remote Repositories might follow in the future.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## License

This project is open source and available under the MIT License.

