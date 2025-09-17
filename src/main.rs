use crate::versions::VersionManager;

mod versions;
mod assets;
mod libraries;
mod downloads;

// TODO: Manage when JSON is ill-formed or missing fields

#[tokio::main]
async fn main() {
    let mut version_manager = VersionManager {
        versions: None,
        latest: None,
    };
    tokio_scoped::scope(|s| {
        s.spawn(async {
            if !version_manager.populate().await {
                println!("Failed to populate version manager");
            }
        });
    });
    if version_manager.versions.is_none() || version_manager.latest.is_none() {
        println!("Version manager is not populated");
        return;
    }
    let mut asset_manager = assets::AssetManager {
        asset_index: None,
    };
    tokio_scoped::scope(|s| {
        s.spawn(async {
            let latest_version_str = &version_manager.latest.as_ref().unwrap().release;
            let latest_version = match version_manager.get_version(latest_version_str) {
                Some(version) => version,
                None => {
                    println!("Couldn't find latest version in versions list");
                    return;
                }
            };
            if !asset_manager.populate(&latest_version.url).await {
                println!("Failed to populate assets manager");
            }
        });
    });
    if asset_manager.asset_index.is_none() {
        println!("Assets manager is not populated");
        return;
    }
    let mut library_manager = libraries::LibraryManager {
        libraries: None,
    };
    tokio_scoped::scope(|s| {
        s.spawn(async {
            let latest_version_str = &version_manager.latest.as_ref().unwrap().release;
            let latest_version = match version_manager.get_version(latest_version_str) {
                Some(version) => version,
                None => {
                    println!("Couldn't find latest version in versions list");
                    return;
                }
            };
            if !library_manager.populate(&latest_version.url).await {
                println!("Failed to populate library manager");
            }
        });
    });
    if library_manager.libraries.is_none() {
        println!("Library manager is not populated");
        return;
    }
    let mut download_manager = downloads::DownloadManager {
        objects: None,
        failed_downloads: None,
    };
    tokio_scoped::scope(|s| {
        s.spawn(async {
            let asset_index_url = &asset_manager.asset_index.as_ref().unwrap().url;
            if !download_manager.populate(asset_index_url).await {
                println!("Failed to populate download manager");
            }
        });
    });
    if download_manager.objects.is_none() {
        println!("Download manager is not populated");
        return;
    }
    for object in download_manager.objects.as_ref().unwrap() {
        println!("Object: {} (hash: {}, size: {})", object.name, object.hash, object.size);
    }
}
