use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub timestamp: u64,
    pub timezone: String,
}

impl Signature {
    pub fn new(name: String, email: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            name,
            email,
            timestamp,
            timezone: "+0000".to_string(),
        }
    }

    pub fn parse(input: &str) -> io::Result<Signature> {
        // Format: "Name <email> timestamp timezone"
        let mut parts = input.rsplitn(3, ' ');
        
        let timezone = parts.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing timezone"))?
            .to_string();

        let timestamp_str = parts.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing timestamp"))?;
        let timestamp = timestamp_str.parse::<u64>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid timestamp"))?;

        let name_email = parts.next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing name and email"))?;

        let email_start = name_email.rfind('<')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing email start"))?;
        let email_end = name_email.rfind('>')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing email end"))?;

        if email_start >= email_end {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid email format"));
        }

        let name = name_email[..email_start].trim().to_string();
        let email = name_email[email_start + 1..email_end].to_string();

        Ok(Signature {
            name,
            email,
            timestamp,
            timezone,
        })
    }

    pub fn format(&self) -> String {
        format!("{} <{}> {} {}", self.name, self.email, self.timestamp, self.timezone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_parse() {
        let input = "John Doe <john@example.com> 1234567890 +0000";
        let sig = Signature::parse(input).unwrap();
        assert_eq!(sig.name, "John Doe");
        assert_eq!(sig.email, "john@example.com");
        assert_eq!(sig.timestamp, 1234567890);
        assert_eq!(sig.timezone, "+0000");
    }

    #[test]
    fn test_signature_format() {
        let sig = Signature {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            timestamp: 1234567890,
            timezone: "+0000".to_string(),
        };
        assert_eq!(sig.format(), "John Doe <john@example.com> 1234567890 +0000");
    }
} 