pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug)]
pub enum OS {
    Windows,
    Linux,
    MacOS,
}

pub struct Rule {
    pub action: String,
    pub os: OS,
}

pub struct Library {
    pub name: String,
    pub artifact: Artifact,
    pub rules: Option<Vec<Rule>>,
}

pub struct LibraryManager {
    pub libraries: Option<Vec<Library>>,
}

impl LibraryManager {
    pub async fn populate(&mut self, version_url: &str) -> bool {
        let version_data = match self.retrieve_version_data(version_url).await {
            Ok(data) => data,
            Err(_) => return false,
        };
        self.parse_libraries(&version_data);
        return true;
    }

    pub async fn retrieve_version_data(&self, version_url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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

    pub fn parse_libraries(&mut self, version_data: &serde_json::Value) {
        let empty = Vec::new();
        let libraries_data = version_data["libraries"].as_array().unwrap_or(&empty);
        let mut libraries: Vec<Library> = vec![];
        for lib in libraries_data {
            let name = lib["name"].as_str().unwrap_or("").to_string();
            let artifact_data = &lib["downloads"]["artifact"];
            let artifact = Artifact {
                path: artifact_data["path"].as_str().unwrap_or("").to_string(),
                sha1: artifact_data["sha1"].as_str().unwrap_or("").to_string(),
                size: artifact_data["size"].as_u64().unwrap_or(0),
                url: artifact_data["url"].as_str().unwrap_or("").to_string(),
            };
            let rules_data = lib["rules"].as_array();
            let mut rules: Option<Vec<Rule>> = None;
            if let Some(rules_array) = rules_data {
                let mut parsed_rules: Vec<Rule> = vec![];
                for rule in rules_array {
                    let action = rule["action"].as_str().unwrap_or("").to_string();
                    let os_data = &rule["os"];
                    let os = match os_data["name"].as_str().unwrap_or("") {
                        "windows" => OS::Windows,
                        "linux" => OS::Linux,
                        "osx" => OS::MacOS,
                        _ => continue,
                    };
                    parsed_rules.push(Rule { action, os });
                }
                if !parsed_rules.is_empty() {
                    rules = Some(parsed_rules);
                }
            }
            libraries.push(Library { name, artifact, rules });
        }
        self.libraries = Some(libraries);
    }

    pub fn get_libraries(&self) -> &Option<Vec<Library>> {
        return &self.libraries;
    }
}