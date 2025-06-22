# Cobra

Cobra is a Git-like version control system implemented in Rust. It provides basic version control functionality similar to Git, allowing you to track changes in your codebase, create commits, and manage your project's history.

## Features

- Repository initialization
- File staging (index)
- Commit creation
- Status checking
- Commit history viewing
- Tree and blob object handling

## Installation

1. Make sure you have Rust and Cargo installed
2. Clone this repository
3. Build the project: `cargo build --release`
4. The binary will be available at `target/release/cobra`

## Commands

```bash
$ cobra -h
A Git implementation in Rust

Usage: cobra [COMMAND]

Commands:
  init      Create an empty Cobra repository
  add       Add file contents to the index
  commit    Record changes to the repository
  log       Show commit logs
  status    Show the working tree status
  branch    Branch operations
  checkout  Switch branches or restore files
  rebase    Reapply commits on top of another base tip
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
raphaele@macbook:~/Projects/cobra$ 
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

## Development Status

This is a learning project that implements core Git functionality. While it's functional for basic version control tasks, it's not intended for production use. Some features that are currently implemented:

- [x] Basic repository operations
- [x] File staging and committing
- [x] Commit history
- [x] Status checking
- [x] Object storage (blobs, trees, commits)
- [x] Branch
- [ ] Remote repository support
- [ ] Stashing

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## License

This project is open source and available under the MIT License.

