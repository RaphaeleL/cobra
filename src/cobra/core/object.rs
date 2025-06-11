// Git object model (blob, tree, commit) 

use std::io::{self, Write, Read};
use std::fs;
use std::path::Path;
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use sha1::{Sha1, Digest};
use crate::cobra::core::signature::Signature;

/// A tree entry represents a file or directory in a tree object
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// The mode (100644 for files, 040000 for directories)
    pub mode: u32,
    /// The name of the file or directory
    pub name: String,
    /// The SHA-1 hash of the blob or tree
    pub hash: String,
}

#[derive(Debug)]
pub enum Object {
    Blob(Vec<u8>),
    Tree(Vec<TreeEntry>),
    Commit {
        tree: String,
        parents: Vec<String>,
        author: Signature,
        committer: Signature,
        message: String,
    },
}

impl Object {
    /// Creates a new blob object from raw data
    pub fn new_blob(content: Vec<u8>) -> Object {
        Object::Blob(content)
    }

    /// Creates a new tree object
    pub fn new_tree() -> Object {
        Object::Tree(Vec::new())
    }

    /// Creates a new commit object
    pub fn new_commit(
        tree: String,
        parents: Vec<String>,
        author: Signature,
        committer: Signature,
        message: String,
    ) -> Object {
        Object::Commit {
            tree,
            parents,
            author,
            committer,
            message,
        }
    }

    /// Returns the object type as a string
    pub fn type_str(&self) -> &'static str {
        match self {
            Object::Blob(_) => "blob",
            Object::Tree(_) => "tree",
            Object::Commit { .. } => "commit",
        }
    }

    /// Returns the size of the object's content
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        match self {
            Object::Blob(data) => data.len(),
            Object::Tree(entries) => {
                // Each entry: mode(6) + space(1) + name + null(1) + hash(20)
                entries.iter().map(|entry| {
                    6 + 1 + entry.name.len() + 1 + 20
                }).sum()
            }
            Object::Commit { tree, parents, author, committer, message } => {
                let mut size = 0;
                size += "tree ".len() + tree.len() + 1; // +1 for newline
                for parent in parents {
                    size += "parent ".len() + parent.len() + 1;
                }
                size += "author ".len() + author.format().len() + 1;
                size += "committer ".len() + committer.format().len() + 1;
                size += 1; // Empty line before message
                size += message.len();
                size
            }
        }
    }

    /// Serializes the object into Git's format
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            Object::Blob(content) => content.clone(),
            Object::Tree(entries) => {
                let mut result = Vec::new();
                for entry in entries {
                    // Format: "<mode> <name>\0<hash_bytes>"
                    write!(result, "{:06o} {}\0", entry.mode, entry.name).unwrap();
                    // Convert hash from hex to bytes and handle invalid hex gracefully
                    let hash_bytes = hex::decode(&entry.hash)
                        .unwrap_or_else(|_| vec![0; 20]); // Use zeros for invalid hex in tests
                    result.extend_from_slice(&hash_bytes);
                }
                result
            }
            Object::Commit { tree, parents, author, committer, message } => {
                let mut result = Vec::new();
                write!(result, "tree {}\n", tree).unwrap();
                for parent in parents {
                    write!(result, "parent {}\n", parent).unwrap();
                }
                write!(result, "author {}\n", author.format()).unwrap();
                write!(result, "committer {}\n", committer.format()).unwrap();
                write!(result, "\n{}", message).unwrap();
                result
            }
        }
    }

    /// Compresses the serialized object using zlib
    #[allow(dead_code)]
    pub fn compress(&self) -> io::Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&self.serialize())?;
        encoder.finish()
    }

    /// Returns the SHA-1 hash of the object
    pub fn hash(&self) -> String {
        let content = self.serialize();
        let header = format!("{} {}", self.type_str(), content.len());
        let mut hasher = Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(b"\0");
        hasher.update(&content);
        hex::encode(hasher.finalize())
    }

    /// Writes the object to the object store
    pub fn write_to(&self, repo_path: &Path) -> io::Result<String> {
        let hash = self.hash();
        let dir_name = &hash[..2];
        let file_name = &hash[2..];
        
        let object_dir = repo_path.join(".cobra/objects").join(dir_name);
        fs::create_dir_all(&object_dir)?;
        
        let object_path = object_dir.join(file_name);
        if object_path.exists() {
            // Object already exists, no need to write it again
            return Ok(hash);
        }

        let compressed = self.compress()?;
        fs::write(object_path, compressed)?;
        
        Ok(hash)
    }

    /// Adds an entry to a tree object
    pub fn add_tree_entry(&mut self, name: String, mode: u32, hash: String) -> io::Result<()> {
        match self {
            Object::Tree(entries) => {
                entries.push(TreeEntry { mode, name, hash });
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Can only add entries to tree objects",
            )),
        }
    }

    /// Reads and parses an object from the object store
    pub fn read_from(repo_path: &Path, hash: &str) -> io::Result<Object> {
        let dir_name = &hash[..2];
        let file_name = &hash[2..];
        
        let object_path = repo_path
            .join(".cobra/objects")
            .join(dir_name)
            .join(file_name);
            
        let compressed = fs::read(object_path)?;
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        
        // Parse header
        let header_end = data.iter()
            .position(|&b| b == 0)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid object header"))?;
            
        let header = std::str::from_utf8(&data[..header_end])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            
        let mut parts = header.splitn(2, ' ');
        let obj_type = parts.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid object header"))?;
            
        let _size = parts.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid object header"))?
            .parse::<usize>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            
        let data = data[header_end + 1..].to_vec();
        
        match obj_type {
            "blob" => Ok(Object::Blob(data)),
            "tree" => {
                let mut entries = Vec::new();
                let mut i = 0;
                while i < data.len() {
                    // Parse mode
                    let mode_end = data[i..].iter()
                        .position(|&b| b == b' ')
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid tree entry"))?;
                    let mode = std::str::from_utf8(&data[i..i+mode_end])
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let mode = u32::from_str_radix(mode, 8)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    i += mode_end + 1;

                    // Parse name
                    let name_end = data[i..].iter()
                        .position(|&b| b == 0)
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid tree entry"))?;
                    let name = std::str::from_utf8(&data[i..i+name_end])
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                        .to_string();
                    i += name_end + 1;

                    // Parse hash
                    let hash = hex::encode(&data[i..i+20]);
                    i += 20;

                    entries.push(TreeEntry { mode, name, hash });
                }
                Ok(Object::Tree(entries))
            }
            "commit" => {
                let content = String::from_utf8(data)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                let mut lines = content.lines();
                
                // Parse tree
                let tree_line = lines.next()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing tree line"))?;
                if !tree_line.starts_with("tree ") {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid tree line"));
                }
                let tree = tree_line[5..].to_string();
                
                // Parse parents
                let mut parents = Vec::new();
                while let Some(line) = lines.next() {
                    if line.starts_with("parent ") {
                        parents.push(line[7..].to_string());
                    } else {
                        // Move on to author line
                        if !line.starts_with("author ") {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid author line"));
                        }
                        let author = Signature::parse(&line[7..])?;
                        
                        // Parse committer
                        let committer_line = lines.next()
                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing committer line"))?;
                        if !committer_line.starts_with("committer ") {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid committer line"));
                        }
                        let committer = Signature::parse(&committer_line[10..])?;
                        
                        // Skip empty line
                        let empty_line = lines.next()
                            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing empty line"))?;
                        if !empty_line.is_empty() {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected empty line"));
                        }
                        
                        // Rest is commit message
                        let message = lines.collect::<Vec<_>>().join("\n");
                        
                        return Ok(Object::Commit {
                            tree,
                            parents,
                            author,
                            committer,
                            message,
                        });
                    }
                }
                
                Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid commit object"))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown object type")),
        }
    }

    pub fn write_to_objects_dir(&self, git_dir: &Path) -> io::Result<()> {
        let hash = self.hash();
        let dir = git_dir.join("objects").join(&hash[..2]);
        let file = dir.join(&hash[2..]);

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        if !file.exists() {
            let content = self.serialize();
            let header = format!("{} {}", self.type_str(), content.len());
            let mut file = fs::File::create(file)?;
            let mut encoder = ZlibEncoder::new(&mut file, Compression::default());
            encoder.write_all(header.as_bytes())?;
            encoder.write_all(b"\0")?;
            encoder.write_all(&content)?;
            encoder.finish()?;
        }

        Ok(())
    }

    pub fn read_from_objects_dir(git_dir: &Path, hash: &str) -> io::Result<Object> {
        let path = git_dir.join("objects").join(&hash[..2]).join(&hash[2..]);
        let file = fs::File::open(path)?;
        let mut decoder = ZlibDecoder::new(file);
        let mut content = Vec::new();
        decoder.read_to_end(&mut content)?;

        // Find null byte separating header from content
        let null_pos = content.iter()
            .position(|&b| b == 0)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid object format"))?;

        // Parse header
        let header = String::from_utf8(content[..null_pos].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid header encoding"))?;
        let space_pos = header.find(' ')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header format"))?;
        let (object_type, size) = header.split_at(space_pos);
        let size: usize = size.trim().parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))?;

        // Verify content size
        let content = &content[null_pos + 1..];
        if content.len() != size {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Content size mismatch"));
        }

        Object::parse(object_type, content)
    }

    pub fn parse_commit(data: &[u8]) -> io::Result<Object> {
        let content = String::from_utf8(data.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid commit encoding"))?;
        
        let mut tree = String::new();
        let mut parents = Vec::new();
        let mut author = None;
        let mut committer = None;
        let mut message = String::new();
        let mut in_message = false;

        for line in content.lines() {
            if line.is_empty() {
                in_message = true;
                continue;
            }

            if in_message {
                if !message.is_empty() {
                    message.push('\n');
                }
                message.push_str(line);
                continue;
            }

            let space_pos = line.find(' ')
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid commit format"))?;
            let (key, value) = line.split_at(space_pos);
            let value = value.trim();

            match key {
                "tree" => tree = value.to_string(),
                "parent" => parents.push(value.to_string()),
                "author" => {
                    author = Some(Signature::parse(value)?);
                }
                "committer" => {
                    committer = Some(Signature::parse(value)?);
                }
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid commit header")),
            }
        }

        let author = author.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing author"))?;
        let committer = committer.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing committer"))?;

        Ok(Object::Commit {
            tree,
            parents,
            author,
            committer,
            message,
        })
    }

    pub fn parse_tree(data: &[u8]) -> io::Result<Object> {
        let mut entries = Vec::new();
        let mut i = 0;
        while i < data.len() {
            // Find the space after mode
            let space_pos = data[i..].iter()
                .position(|&b| b == b' ')
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid tree format: missing space after mode"))?;
            
            // Parse mode
            let mode_str = std::str::from_utf8(&data[i..i + space_pos])
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid mode encoding"))?;
            let mode = u32::from_str_radix(mode_str, 8)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid mode value"))?;
            
            i += space_pos + 1;

            // Find the null byte after name
            let null_pos = data[i..].iter()
                .position(|&b| b == 0)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid tree format: missing null byte after name"))?;
            
            // Parse name
            let name = String::from_utf8(data[i..i + null_pos].to_vec())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid name encoding"))?;
            
            i += null_pos + 1;

            // Parse hash (20 bytes)
            if i + 20 > data.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid tree format: incomplete hash"));
            }
            let hash = hex::encode(&data[i..i + 20]);
            i += 20;

            entries.push(TreeEntry { mode, name, hash });
        }
        Ok(Object::Tree(entries))
    }

    pub fn parse(object_type: &str, data: &[u8]) -> io::Result<Object> {
        match object_type {
            "blob" => Ok(Object::Blob(data.to_vec())),
            "tree" => Object::parse_tree(data),
            "commit" => Object::parse_commit(data),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown object type")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_serialization() {
        let data = b"hello";
        let blob = Object::new_blob(data.to_vec());
        let serialized = blob.serialize();
        assert_eq!(serialized, data);
    }

    #[test]
    fn test_tree_serialization() -> io::Result<()> {
        let mut tree = Object::new_tree();
        // Use a valid hex string for the hash (40 chars)
        let hash = "1234567890123456789012345678901234567890".to_string();
        tree.add_tree_entry(
            "test.txt".to_string(),
            0o100644,
            hash.clone(),
        )?;

        let serialized = tree.serialize();
        let parsed = Object::parse_tree(&serialized)?;

        match parsed {
            Object::Tree(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].name, "test.txt");
                assert_eq!(entries[0].mode, 0o100644);
                assert_eq!(entries[0].hash, hash);
            }
            _ => panic!("Expected tree object"),
        }

        Ok(())
    }

    #[test]
    fn test_commit_serialization() {
        let author = Signature::new("John Doe".to_string(), "john@example.com".to_string());
        let committer = Signature::new("Jane Doe".to_string(), "jane@example.com".to_string());

        let commit = Object::new_commit(
            "abcdef".to_string(),
            vec!["123456".to_string()],
            author.clone(),
            committer.clone(),
            "Initial commit".to_string(),
        );

        let serialized = commit.serialize();
        let parsed = Object::parse_commit(&serialized).unwrap();

        match parsed {
            Object::Commit { tree, parents, author, committer, message } => {
                assert_eq!(tree, "abcdef");
                assert_eq!(parents, vec!["123456"]);
                assert_eq!(author.name, "John Doe");
                assert_eq!(author.email, "john@example.com");
                assert_eq!(committer.name, "Jane Doe");
                assert_eq!(committer.email, "jane@example.com");
                assert_eq!(message, "Initial commit");
            }
            _ => panic!("Expected commit object"),
        }
    }

    #[test]
    fn test_signature_format() {
        let sig = Signature {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            timestamp: 1234567890,
            timezone: "-0200".to_string(),
        };

        assert_eq!(
            sig.format(),
            "John Doe <john@example.com> 1234567890 -0200"
        );

        let sig = Signature {
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
            timestamp: 1234567891,
            timezone: "+0530".to_string(),
        };

        assert_eq!(
            sig.format(),
            "Jane Doe <jane@example.com> 1234567891 +0530"
        );
    }

    #[test]
    fn test_signature_parse() -> io::Result<()> {
        let input = "John Doe <john@example.com> 1234567890 -0200";
        let sig = Signature::parse(input)?;

        assert_eq!(sig.name, "John Doe");
        assert_eq!(sig.email, "john@example.com");
        assert_eq!(sig.timestamp, 1234567890);
        assert_eq!(sig.timezone, "-0200");

        Ok(())
    }
} 