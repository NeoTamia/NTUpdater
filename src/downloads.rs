use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha1::{Digest, Sha1};
use tokio::sync::Semaphore;

const DOWNLOAD_BASE_URL: &str = "https://resources.download.minecraft.net";
const MAX_CONCURRENT_DOWNLOADS: usize = 32;
const MAX_RETRIES: usize = 3;

pub struct Object {
    pub name: String,
    pub hash: String,
    pub size: i64,
}

pub struct DownloadManager {
    pub objects: Option<Vec<Object>>,
    pub failed_downloads: Option<Vec<String>>,
}

impl DownloadManager {
    pub async fn populate(&mut self, asset_index_url: &str) -> bool {
        let asset_index_data = match self.retrieve_asset_index_data(asset_index_url).await {
            Ok(data) => data,
            Err(_) => return false,
        };
        self.parse_objects(&asset_index_data);
        return true;
    }

    pub async fn retrieve_asset_index_data(&self, asset_index_url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = match reqwest::get(asset_index_url).await {
            Ok(response) => response,
            Err(e) => return Err(e.into()),
        };
        let asset_index_data: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => return Err(e.into()),
        };
        return Ok(asset_index_data)
    }

    pub fn parse_objects(&mut self, asset_index_data: &serde_json::Value) {
        let objects_data = &asset_index_data["objects"];
        let mut objects: Vec<Object> = vec![];
        for object in objects_data.as_object().unwrap() {
            objects.push(
                Object {
                    name: object.0.to_string(),
                    hash: object.1["hash"].as_str().unwrap_or("").to_string(),
                    size: object.1["size"].as_i64().unwrap_or(0),
                }
            )
        }
        self.objects = Option::from(objects);
    }

    pub fn get_objects(&self) -> &Option<Vec<Object>> {
        return &self.objects;
    }

    /// Download every parsed object into `objects_dir`, laid out as
    /// `objects_dir/<first two hash chars>/<hash>` (the Minecraft assets layout).
    ///
    /// Downloads run concurrently, capped by a semaphore. Each object is
    /// SHA-1 verified; files that are already present and valid are skipped so
    /// re-running the updater is cheap. Names that could not be downloaded end
    /// up in `failed_downloads`. Returns `true` when every object succeeded.
    pub async fn download_all(&mut self, objects_dir: &Path) -> bool {
        let objects = match &self.objects {
            Some(objects) => objects,
            None => return false,
        };

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
        let failed = Arc::new(Mutex::new(Vec::<String>::new()));
        // One shared Client -> one connection pool. Reusing it across every
        // download keeps TCP/TLS connections warm instead of paying a fresh
        // handshake per file (what reqwest::get() would do).
        let client = reqwest::Client::new();

        tokio_scoped::scope(|s| {
            for object in objects {
                let semaphore = Arc::clone(&semaphore);
                let failed = Arc::clone(&failed);
                let client = &client;
                s.spawn(async move {
                    // Held for the whole download so at most
                    // MAX_CONCURRENT_DOWNLOADS run at once.
                    let _permit = semaphore.acquire().await.unwrap();
                    if let Err(e) = download_object(client, object, objects_dir).await {
                        eprintln!("Failed to download {}: {}", object.name, e);
                        failed.lock().unwrap().push(object.name.clone());
                    }
                });
            }
        });

        let failed = Arc::try_unwrap(failed).unwrap().into_inner().unwrap();
        let all_ok = failed.is_empty();
        self.failed_downloads = Some(failed);
        return all_ok;
    }
}

/// Download a single object with retry + SHA-1 verification. A file already on
/// disk with the expected hash is left untouched.
async fn download_object(
    client: &reqwest::Client,
    object: &Object,
    objects_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if object.hash.len() < 2 {
        return Err(format!("invalid hash for object {}", object.name).into());
    }
    let sub = &object.hash[0..2];
    let dest = objects_dir.join(sub).join(&object.hash);

    // Already present and valid -> nothing to do.
    if let Ok(existing) = tokio::fs::read(&dest).await {
        if sha1_hex(&existing) == object.hash {
            return Ok(());
        }
    }

    let url = format!("{}/{}/{}", DOWNLOAD_BASE_URL, sub, object.hash);
    let mut last_err: Option<Box<dyn Error + Send + Sync>> = None;
    for attempt in 1..=MAX_RETRIES {
        match fetch_verified(client, &url, &object.hash).await {
            Ok(bytes) => {
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&dest, &bytes).await?;
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                if attempt < MAX_RETRIES {
                    // Linear backoff between attempts.
                    tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                }
            }
        }
    }
    return Err(last_err.unwrap_or_else(|| "download failed".into()));
}

/// Fetch `url` and return its bytes only if they hash to `expected_sha1`.
async fn fetch_verified(
    client: &reqwest::Client,
    url: &str,
    expected_sha1: &str,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), url).into());
    }
    let bytes = response.bytes().await?.to_vec();
    let actual = sha1_hex(&bytes);
    if actual != expected_sha1 {
        return Err(format!(
            "sha1 mismatch for {}: expected {}, got {}",
            url, expected_sha1, actual
        )
        .into());
    }
    return Ok(bytes);
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    return hex;
}