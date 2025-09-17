use std::error::Error;

pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: u64,
    pub url: String,
}

pub struct AssetManager {
    pub asset_index: Option<AssetIndex>,
}

impl AssetManager {
    pub async fn populate(&mut self, version_url: &str) -> bool {
        let version_data = match self.retrieve_version_data(version_url).await {
            Ok(data) => data,
            Err(_) => return false,
        };
        self.parse_asset_index(&version_data);
        return true;
    }

    pub async fn retrieve_version_data(&self, version_url: &str) -> Result<serde_json::Value, Box<dyn Error>> {
        let response = match reqwest::get(version_url).await {
            Ok(response) => response,
            Err(e) => return Err(e.into()),
        };
        let version_data: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => return Err(e.into()),
        };
        return Ok(version_data)
    }

    pub fn parse_asset_index(&mut self, version_data: &serde_json::Value) {
        let asset_index_data = &version_data["assetIndex"];
        self.asset_index = Option::from(AssetIndex {
            id: asset_index_data["id"].as_str().unwrap_or("").to_string(),
            sha1: asset_index_data["sha1"].as_str().unwrap_or("").to_string(),
            size: asset_index_data["size"].as_u64().unwrap_or(0),
            total_size: asset_index_data["totalSize"].as_u64().unwrap_or(0),
            url: asset_index_data["url"].as_str().unwrap_or("").to_string(),
        });
    }
    
    pub fn get_asset_index(&self) -> &Option<AssetIndex> {
        return &self.asset_index;
    }
}