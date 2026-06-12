use crate::error::{VaultError, VaultResult};

#[derive(Debug, Clone, PartialEq)]
pub enum UriScheme {
    S3,
    Dropbox,
    GoogleDrive,
    Local,
}

impl UriScheme {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::S3          => "s3",
            Self::Dropbox     => "drop",
            Self::GoogleDrive => "drive",
            Self::Local       => "fs",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StorageUri {
    pub scheme:        UriScheme,
    pub bucket_slug:   String,
    pub relative_path: String,
    pub file_name:     String,
}

impl StorageUri {
    pub fn parse(raw: &str) -> VaultResult<Self> {
        let (scheme_str, rest) = raw
            .split_once("://")
            .ok_or_else(|| VaultError::InvalidUri(raw.to_string()))?;

        let scheme = match scheme_str {
            "s3"    => UriScheme::S3,
            "drop"  => UriScheme::Dropbox,
            "drive" => UriScheme::GoogleDrive,
            "fs"    => UriScheme::Local,
            other   => return Err(VaultError::InvalidUri(format!("Unknown scheme: {other}"))),
        };

        let (bucket_slug, relative_path) = rest
            .split_once('/')
            .map(|(b, p)| (b.to_string(), p.to_string()))
            .unwrap_or_else(|| (rest.to_string(), String::new()));

        let file_name = relative_path
            .rsplit('/')
            .next()
            .unwrap_or(&relative_path)
            .to_string();

        Ok(StorageUri { scheme, bucket_slug, relative_path, file_name })
    }

    #[allow(dead_code)]
    pub fn to_uri_string(&self) -> String {
        format!("{}://{}/{}", self.scheme.as_str(), self.bucket_slug, self.relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_s3_uri() {
        let u = StorageUri::parse("s3://my-bucket/2024/file.csv").unwrap();
        assert_eq!(u.scheme, UriScheme::S3);
        assert_eq!(u.bucket_slug, "my-bucket");
        assert_eq!(u.relative_path, "2024/file.csv");
        assert_eq!(u.file_name, "file.csv");
    }

    #[test]
    fn parse_local_uri() {
        let u = StorageUri::parse("fs://vx-bucket-01/a/b/c.csv").unwrap();
        assert_eq!(u.scheme, UriScheme::Local);
        assert_eq!(u.bucket_slug, "vx-bucket-01");
        assert_eq!(u.relative_path, "a/b/c.csv");
        assert_eq!(u.file_name, "c.csv");
    }

    #[test]
    fn parse_dropbox_uri() {
        let u = StorageUri::parse("drop://vx-sat-putin/reports/q1.pdf").unwrap();
        assert_eq!(u.scheme, UriScheme::Dropbox);
        assert_eq!(u.bucket_slug, "vx-sat-putin");
        assert_eq!(u.relative_path, "reports/q1.pdf");
        assert_eq!(u.file_name, "q1.pdf");
    }

    #[test]
    fn parse_drive_uri() {
        let u = StorageUri::parse("drive://vx-sat-ramad/assets/cover.jpg").unwrap();
        assert_eq!(u.scheme, UriScheme::GoogleDrive);
        assert_eq!(u.bucket_slug, "vx-sat-ramad");
        assert_eq!(u.file_name, "cover.jpg");
    }

    #[test]
    fn roundtrip_uri() {
        let raw = "fs://vx-bucket-01/folder-1/folder-2/file.csv";
        let u = StorageUri::parse(raw).unwrap();
        assert_eq!(u.to_uri_string(), raw);
    }

    #[test]
    fn invalid_scheme_returns_error() {
        assert!(StorageUri::parse("ftp://bucket/file.csv").is_err());
    }
}
