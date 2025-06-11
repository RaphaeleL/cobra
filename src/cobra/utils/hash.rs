// SHA-1 hashing utilities 

use sha1::{Sha1, Digest};
use std::fmt::Write;

/// Creates the header for a Git object
fn create_object_header(obj_type: &str, content_len: usize) -> Vec<u8> {
    format!("{} {}\0", obj_type, content_len).into_bytes()
}

/// Computes the SHA-1 hash of data in Git's format
/// Returns the hex string representation of the hash
pub fn hash_object(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    
    let mut hex_string = String::with_capacity(40);
    for byte in result {
        write!(&mut hex_string, "{:02x}", byte).expect("Writing to string cannot fail");
    }
    hex_string
}

/// Computes the SHA-1 hash of raw data, adding a Git object header
/// Returns the hex string representation of the hash
pub fn hash_raw_object(obj_type: &str, data: &[u8]) -> String {
    let header = create_object_header(obj_type, data.len());
    let mut content = Vec::with_capacity(header.len() + data.len());
    content.extend(&header);
    content.extend(data);
    hash_object(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_object() {
        // Test cases with raw SHA-1 hashes (no Git header)
        let test_cases = [
            (b"hello" as &[u8], "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"),
            (b"hello\n" as &[u8], "f572d396fae9206628714fb2ce00f72e94f2258f"),
        ];

        for (input, expected) in test_cases {
            let result = hash_object(input);
            assert_eq!(result, expected, "Failed for input: {:?}", input);
        }
    }

    #[test]
    fn test_hash_raw_object() {
        // Test cases verified with `git hash-object --stdin`
        let test_cases = [
            (b"hello" as &[u8], "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"),
            (b"hello\n" as &[u8], "ce013625030ba8dba906f756967f9e9ca394464a"),
        ];

        for (input, expected) in test_cases {
            let result = hash_raw_object("blob", input);
            assert_eq!(result, expected, "Failed for input: {:?}", input);
        }
    }

    #[test]
    fn test_create_object_header() {
        let header = create_object_header("blob", 5);
        assert_eq!(header, b"blob 5\0");
    }
} 