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

# Print the help message
cobra help

# Initialize a new repository
cobra init [path]

# Add files to staging area
cobra add <file>

# Create a new commit
cobra commit -m "message"

# Show working directory status
cobra status

# Show commit history
cobra log

# Create a branch
cobra branch -c dev

# Switch to a branch 
cobra branch -s dev

# Show all branches 
cobra branch -a

# Delete a branch 
cobra branch -d dev

# Merge 'dev' into the current branch 
cobra branch -m dev
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

