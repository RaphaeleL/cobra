use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write, Read};
use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};

use super::repository::Repository;

#[allow(dead_code)]
const SIGNATURE: &[u8; 4] = b"COBA"; // Our index signature
#[allow(dead_code)]
const VERSION: u32 = 1; // Index format version

/// Represents a single entry in the index
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// The time the file was last modified
    pub ctime: u64,
    /// The time the file was last modified
    pub mtime: u64,
    /// The device number
    pub dev: u32,
    /// The inode number
    pub ino: u32,
    /// The file mode (permissions)
    pub mode: u32,
    /// The user ID
    pub uid: u32,
    /// The group ID
    pub gid: u32,
    /// The file size
    pub size: u64,
    /// The SHA-1 hash of the file content
    pub hash: String,
    /// The path of the file relative to repository root
    pub path: PathBuf,
}

impl IndexEntry {
    /// Creates a new index entry from a file
    pub fn new(path: PathBuf, hash: String, metadata: fs::Metadata) -> IndexEntry {
        IndexEntry {
            ctime: metadata.ctime() as u64,
            mtime: metadata.mtime() as u64,
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode: metadata.mode() as u32,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.len(),
            hash,
            path,
        }
    }

    /// Write entry to a binary format
    fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Write fixed-length fields
        writer.write_u64::<BigEndian>(self.ctime)?;
        writer.write_u64::<BigEndian>(self.mtime)?;
        writer.write_u32::<BigEndian>(self.dev)?;
        writer.write_u32::<BigEndian>(self.ino)?;
        writer.write_u32::<BigEndian>(self.mode)?;
        writer.write_u32::<BigEndian>(self.uid)?;
        writer.write_u32::<BigEndian>(self.gid)?;
        writer.write_u64::<BigEndian>(self.size)?;

        // Write hash
        writer.write_all(self.hash.as_bytes())?;
        writer.write_u8(0)?; // Null terminator

        // Write path
        let path_str = self.path.to_string_lossy();
        writer.write_all(path_str.as_bytes())?;
        writer.write_u8(0)?; // Null terminator

        Ok(())
    }

    /// Read entry from a binary format
    fn read_from<R: Read>(reader: &mut R) -> io::Result<IndexEntry> {
        // Read fixed-length fields
        let ctime = reader.read_u64::<BigEndian>()?;
        let mtime = reader.read_u64::<BigEndian>()?;
        let dev = reader.read_u32::<BigEndian>()?;
        let ino = reader.read_u32::<BigEndian>()?;
        let mode = reader.read_u32::<BigEndian>()?;
        let uid = reader.read_u32::<BigEndian>()?;
        let gid = reader.read_u32::<BigEndian>()?;
        let size = reader.read_u64::<BigEndian>()?;

        // Read hash (null-terminated string)
        let mut hash = Vec::new();
        loop {
            let byte = reader.read_u8()?;
            if byte == 0 {
                break;
            }
            hash.push(byte);
        }
        let hash = String::from_utf8(hash)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Read path (null-terminated string)
        let mut path = Vec::new();
        loop {
            let byte = reader.read_u8()?;
            if byte == 0 {
                break;
            }
            path.push(byte);
        }
        let path = String::from_utf8(path)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(IndexEntry {
            ctime,
            mtime,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            hash,
            path: PathBuf::from(path),
        })
    }
}

/// Represents the index (staging area)
#[derive(Debug, Default)]
pub struct Index {
    /// Map of paths to index entries
    entries: Vec<IndexEntry>,
}

impl Index {
    /// Creates a new empty index
    pub fn new() -> Index {
        Index {
            entries: Vec::new(),
        }
    }

    /// Loads the index from the repository
    pub fn load(repo: &Repository) -> io::Result<Index> {
        let index_path = repo.git_dir.join("index");
        if index_path.exists() {
            Index::read_from_file(&index_path)
        } else {
            Ok(Index::new())
        }
    }

    /// Adds or updates an entry in the index
    pub fn add_entry(&mut self, entry: IndexEntry) {
        // Remove any existing entry for this path
        self.entries.retain(|e| e.path != entry.path);
        // Add the new entry
        self.entries.push(entry);
    }

    /// Gets an entry from the index by path
    pub fn get_entry(&self, path: &Path) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.path == *path)
    }

    /// Returns true if the path exists in the index
    pub fn contains(&self, path: &Path) -> bool {
        self.entries.iter().any(|e| e.path == *path)
    }

    /// Returns an iterator over all entries
    pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.entries.iter()
    }

    /// Write the index to a file
    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        
        // Write number of entries
        file.write_u32::<BigEndian>(self.entries.len() as u32)?;

        // Write each entry
        for entry in &self.entries {
            entry.write_to(&mut file)?;
        }

        Ok(())
    }

    /// Read the index from a file
    pub fn read_from_file(path: &Path) -> io::Result<Index> {
        let mut file = fs::File::open(path)?;
        
        // Read number of entries
        let num_entries = file.read_u32::<BigEndian>()?;
        
        let mut entries = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            entries.push(IndexEntry::read_from(&mut file)?);
        }

        Ok(Index { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_index_entry_new() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        
        // Create a test file
        let mut file = File::create(&file_path)?;
        writeln!(file, "test content")?;
        
        // Create index entry
        let entry = IndexEntry::new(PathBuf::from("test.txt"), "abcdef".to_string(), fs::metadata(&file_path)?);
        
        // Verify basic properties
        assert_eq!(entry.path, PathBuf::from("test.txt"));
        assert!(entry.size > 0);
        assert!(entry.mode > 0);
        assert!(entry.mtime > 0);
        assert!(!entry.hash.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_index_operations() {
        let mut index = Index::new();
        let entry = IndexEntry {
            ctime: 12345,
            mtime: 12345,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 100,
            hash: "abcdef".to_string(),
            path: PathBuf::from("test.txt"),
        };

        // Test adding entry
        index.add_entry(entry.clone());
        assert!(index.contains(&PathBuf::from("test.txt")));

        // Test getting entry
        let retrieved = index.get_entry(&PathBuf::from("test.txt")).unwrap();
        assert_eq!(retrieved.hash, "abcdef");
        assert_eq!(retrieved.size, 100);

        // Test entries iterator
        let entries: Vec<_> = index.entries().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_index_serialization() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("index");
        
        // Create an index with some entries
        let mut index = Index::new();
        index.add_entry(IndexEntry {
            ctime: 12345,
            mtime: 12345,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 100,
            hash: "a".repeat(40),
            path: PathBuf::from("test1.txt"),
        });
        index.add_entry(IndexEntry {
            ctime: 67890,
            mtime: 67890,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 200,
            hash: "b".repeat(40),
            path: PathBuf::from("test2.txt"),
        });
        
        // Write to file
        index.write_to_file(&index_path)?;
        
        // Read back
        let read_index = Index::read_from_file(&index_path)?;
        
        // Verify entries match
        assert_eq!(read_index.entries.len(), 2);
        assert!(read_index.contains(&PathBuf::from("test1.txt")));
        assert!(read_index.contains(&PathBuf::from("test2.txt")));
        
        let entry1 = read_index.get_entry(&PathBuf::from("test1.txt")).unwrap();
        assert_eq!(entry1.size, 100);
        assert_eq!(entry1.hash, "a".repeat(40));
        
        Ok(())
    }
} 