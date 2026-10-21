use crate::settings::CosSettings;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use sha1::Sha1;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UploadKind {
    Image,
    File,
}

pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_FILE_BYTES: usize = 100 * 1024 * 1024;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "svg", "ico", "tif", "tiff", "heic", "heif",
];

impl UploadKind {
    fn folder(self) -> &'static str {
        match self {
            UploadKind::Image => "images",
            UploadKind::File => "files",
        }
    }

    fn max_bytes(self) -> usize {
        match self {
            UploadKind::Image => MAX_IMAGE_BYTES,
            UploadKind::File => MAX_FILE_BYTES,
        }
    }
}

/// The extension ends up in the object key and the URL pasted into the note, so only short
/// alphanumeric ones are accepted; images must also be a known image type.
pub fn validate_extension(extension: &str, kind: UploadKind) -> Result<String, String> {
    let extension = extension.to_ascii_lowercase();
    let well_formed = (1..=10).contains(&extension.len()) && extension.bytes().all(|b| b.is_ascii_alphanumeric());
    if !well_formed {
        return Err(format!("Unsupported file extension: {:?}", extension));
    }
    if kind == UploadKind::Image && !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!("Not an image type: .{}", extension));
    }
    Ok(extension)
}

pub fn validate_size(len: usize, kind: UploadKind) -> Result<(), String> {
    if len == 0 {
        return Err("The file is empty".to_string());
    }
    if len > kind.max_bytes() {
        return Err(format!(
            "The file is too large ({:.1} MB, limit {} MB)",
            len as f64 / 1024.0 / 1024.0,
            kind.max_bytes() / 1024 / 1024
        ));
    }
    Ok(())
}

fn object_key(prefix: &str, folder: &str, data: &[u8], extension: &str) -> String {
    format!("{}{}/{:x}.{}", prefix, folder, md5::compute(data), extension)
}

#[allow(clippy::too_many_arguments)] // mirrors the fields of the COS signing spec
fn calculate_signature(secret_id: &str, secret_key: &str, method: &str, uri: &str, params: &HashMap<String, String>, headers: &HashMap<String, String>, start_timestamp: i64, end_timestamp: i64) -> String {
    let q_sign_algorithm = "sha1";
    let q_ak = secret_id;
    let q_sign_time = format!("{};{}", start_timestamp, end_timestamp);
    let q_key_time = format!("{};{}", start_timestamp, end_timestamp);
    
    let mut header_keys: Vec<&String> = headers.keys().collect();
    header_keys.sort();
    let q_header_list = header_keys.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(";");
    
    let mut url_param_keys: Vec<&String> = params.keys().collect();
    url_param_keys.sort();
    let q_url_param_list = url_param_keys.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(";");
    
    let format_string = |map: &HashMap<String, String>, keys: &Vec<&String>| -> String {
        keys.iter()
            .map(|k| format!("{}={}", k.to_lowercase(), urlencoding::encode(map.get(*k).unwrap())))
            .collect::<Vec<_>>()
            .join("&")
    };
    
    let http_parameters = format_string(params, &url_param_keys);
    let http_headers = format_string(headers, &header_keys);
    
    let http_string = format!("{}\n{}\n{}\n{}\n", method.to_lowercase(), uri, http_parameters, http_headers);
    
    let string_to_sign = format!("{}\n{}\n{}\n", q_sign_algorithm, q_sign_time, sha1_hex(&http_string));
    
    let sign_key = hmac_sha1(secret_key, &q_key_time);
    let signature = hmac_sha1(&sign_key, &string_to_sign);
    
    format!("q-sign-algorithm={}&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list={}&q-signature={}",
            q_sign_algorithm, q_ak, q_sign_time, q_key_time, q_header_list, q_url_param_list, signature)
}

fn sha1_hex(data: &str) -> String {
    use sha1::Digest;
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn hmac_sha1(key: &str, data: &str) -> String {
    type HmacSha1 = Hmac<Sha1>;
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Uploads to `<prefix><folder>/<md5>.<extension>`, skipping the PUT when that object already exists.
pub fn upload(settings: &CosSettings, file_data: Vec<u8>, extension: &str, kind: UploadKind) -> Result<String, String> {
    let CosSettings { secret_id, secret_key, bucket, region, cdn_domain, prefix } = settings;
    let folder = kind.folder();

    if secret_id.is_empty() || secret_key.is_empty() || bucket.is_empty() || region.is_empty() {
        return Err("COS configuration is missing. Please check settings.".to_string());
    }

    let key = object_key(prefix, folder, &file_data, extension);
    
    // Check if file exists (HEAD request)
    let host = format!("{}.cos.{}.myqcloud.com", bucket, region);
    let uri = format!("/{}", key);
    let url = format!("https://{}{}", host, uri);
    
    let client = Client::new();
    
    // HEAD
    let now = Utc::now();
    let start_timestamp = now.timestamp();
    let end_timestamp = start_timestamp + 600;
    
    let mut headers = HashMap::new();
    headers.insert("Host".to_string(), host.clone());
    
    let signature = calculate_signature(secret_id, secret_key, "HEAD", &uri, &HashMap::new(), &headers, start_timestamp, end_timestamp);
    
    let head_res = client.head(&url)
        .header("Authorization", signature)
        .header("Host", &host)
        .send();

    if let Ok(res) = head_res {
        if res.status().is_success() {
            // File exists
            if cdn_domain.is_empty() {
                return Ok(format!("https://{}/{}", host, key));
            } else {
                return Ok(format!("https://{}/{}", cdn_domain, key));
            }
        }
    }

    // PUT
    let now = Utc::now();
    let start_timestamp = now.timestamp();
    let end_timestamp = start_timestamp + 600;
    let signature = calculate_signature(secret_id, secret_key, "PUT", &uri, &HashMap::new(), &headers, start_timestamp, end_timestamp);

    let res = client.put(&url)
        .header("Authorization", signature)
        .header("Host", &host)
        .body(file_data)
        .send()
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        if cdn_domain.is_empty() {
            Ok(format!("https://{}/{}", host, key))
        } else {
            Ok(format!("https://{}/{}", cdn_domain, key))
        }
    } else {
        Err(format!("Upload failed: {}", res.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_short_alphanumeric_extensions() {
        assert_eq!(validate_extension("PNG", UploadKind::Image).unwrap(), "png");
        assert_eq!(validate_extension("jpeg", UploadKind::Image).unwrap(), "jpeg");
        assert_eq!(validate_extension("pdf", UploadKind::File).unwrap(), "pdf");
        assert_eq!(validate_extension("7z", UploadKind::File).unwrap(), "7z");
    }

    #[test]
    fn rejects_extensions_that_would_break_the_key_or_url() {
        for ext in ["", "tar.gz", "../png", "png/x", "p ng", "pn?g", "abcdefghijk", "图片"] {
            assert!(validate_extension(ext, UploadKind::File).is_err(), "{:?}", ext);
        }
    }

    #[test]
    fn rejects_non_image_extensions_for_images() {
        assert!(validate_extension("exe", UploadKind::Image).is_err());
        assert!(validate_extension("html", UploadKind::Image).is_err());
        assert!(validate_extension("exe", UploadKind::File).is_ok());
    }

    #[test]
    fn enforces_size_limits() {
        assert!(validate_size(0, UploadKind::File).is_err());
        assert!(validate_size(1, UploadKind::Image).is_ok());
        assert!(validate_size(MAX_IMAGE_BYTES, UploadKind::Image).is_ok());
        assert!(validate_size(MAX_IMAGE_BYTES + 1, UploadKind::Image).is_err());
        assert!(validate_size(MAX_IMAGE_BYTES + 1, UploadKind::File).is_ok());
        assert!(validate_size(MAX_FILE_BYTES + 1, UploadKind::File).is_err());
    }

    #[test]
    fn names_objects_by_content_hash() {
        assert_eq!(object_key("press/", "images", b"hello", "png"), "press/images/5d41402abc4b2a76b9719d911017c592.png");
    }

    #[test]
    fn signs_requests_deterministically() {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "bucket-1250000000.cos.ap-shanghai.myqcloud.com".to_string());
        let signature = calculate_signature("AKIDexample", "secret", "PUT", "/press/images/a.png", &HashMap::new(), &headers, 1_700_000_000, 1_700_000_600);
        assert_eq!(
            signature,
            "q-sign-algorithm=sha1&q-ak=AKIDexample&q-sign-time=1700000000;1700000600&q-key-time=1700000000;1700000600&q-header-list=host&q-url-param-list=&q-signature=5b5a0b75edf2f800acde43739d5a75029fe22127"
        );
    }
}
