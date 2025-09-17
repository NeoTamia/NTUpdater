use reqwest;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug)]
pub enum VersionType {
    Snapshot,
    Release,
    OldBeta,
    OldAlpha,
}

impl PartialEq for VersionType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VersionType::Snapshot, VersionType::Snapshot) => true,
            (VersionType::Release, VersionType::Release) => true,
            (VersionType::OldBeta, VersionType::OldBeta) => true,
            (VersionType::OldAlpha, VersionType::OldAlpha) => true,
            _ => false,
        }
    }
}

pub struct LatestVersion {
    pub snapshot: String,
    pub release: String,
}

pub struct Version {
    pub id: String,
    pub version_type: VersionType,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: u8,
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id
            && self.version_type == other.version_type
            && self.url == other.url
            && self.time == other.time
            && self.release_time == other.release_time
            && self.sha1 == other.sha1
            && self.compliance_level == other.compliance_level;
    }
}

pub struct VersionManager {
    pub(crate) versions: Option<Vec<Version>>,
    pub(crate) latest: Option<LatestVersion>,
}

impl VersionManager {
    pub async fn populate(&mut self) -> bool {
        let manifest = match self.retrieve_manifest().await {
            Ok(manifest) => manifest,
            Err(_) => return false,
        };
        self.parse_latest(&manifest);
        self.parse_versions(&manifest);
        return true;
    }

    pub async fn retrieve_manifest(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = match reqwest::get(MANIFEST_URL).await {
            Ok(resp) => resp,
            Err(e) => {
                println!("Couldn't retrieve manifest: {}", e);
                return Err(e.into());
            }
        };
        let manifest: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                println!("Couldn't parse manifest: {}", e);
                return Err(e.into());
            }
        };
        return Ok(manifest);
    }

    pub fn parse_latest(&mut self, manifest: &serde_json::Value) {
        let latest = &manifest["latest"];
        self.latest = Option::from(LatestVersion {
            snapshot: latest["snapshot"].as_str().unwrap_or("").to_string(),
            release: latest["release"].as_str().unwrap_or("").to_string(),
        });
    }

    pub fn parse_versions(&mut self, manifest: &serde_json::Value) {
        let empty = Vec::new();
        let versions = manifest["versions"].as_array().unwrap_or(&empty);
        self.versions = Option::from(Vec::new());
        for version in versions {
            let id = version["id"].as_str().unwrap_or("").to_string();
            let version_type_str = version["type"].as_str().unwrap_or("");
            let version_type = match version_type_str {
                "snapshot" => VersionType::Snapshot,
                "release" => VersionType::Release,
                "old_beta" => VersionType::OldBeta,
                "old_alpha" => VersionType::OldAlpha,
                _ => continue,
            };
            let url = version["url"].as_str().unwrap_or("").to_string();
            let time = version["time"].as_str().unwrap_or("").to_string();
            let release_time = version["releaseTime"].as_str().unwrap_or("").to_string();
            let sha1 = version["sha1"].as_str().unwrap_or("").to_string();
            let compliance_level = version["complianceLevel"].as_u64().unwrap_or(0) as u8;
            self.versions.as_mut().unwrap().push(
                Version {
                    id,
                    version_type,
                    url,
                    time,
                    release_time,
                    sha1,
                    compliance_level,
                }
            )
        }
    }

    // TODO: Add all getter and return copies so the internal state can't be modified
    pub fn get_version(&self, id: &str) -> Option<&Version> {
        return match &self.versions {
            Some(versions) => {
                versions.iter().find(|&v| v.id == id)
            },
            None => None,
        }
    }
}