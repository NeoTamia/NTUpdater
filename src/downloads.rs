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
}