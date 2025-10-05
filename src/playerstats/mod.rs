use bollard::Docker;
use bollard::container::DownloadFromContainerOptions;
use futures_util::stream::TryStreamExt;
use std::collections::HashMap;
use std::io::Cursor;
use bollard::query_parameters::InspectContainerOptions;
use log::warn;
use tar::Archive;
use serde_json::Value;

pub mod minecraft_players;
mod cobblemon_stats;

pub struct DockerFetcher {
    docker: Docker,
}

impl DockerFetcher {
    pub fn new() -> Self {
        let docker = Docker::connect_with_unix_defaults().expect("Impossible to connect to docker");
        DockerFetcher { docker }
    }

    /// Fetch all JSON file of a certain file path in the container
    /// # Parameters
    /// - `container_name`: Container's name or ID
    /// - `remote_path`: Path inside the container (ex: `/data/world/stats`)
    /// # Returns
    /// - Return a HashMap { "uuid" => JSON Value }
    pub async fn fetch_json_files(
        &self,
        container_name: &str,
        remote_path: &str,
    ) -> anyhow::Result<HashMap<String, Value>> {
        let options = DownloadFromContainerOptions {
            path: remote_path.to_string(),
        };

        if self.docker.inspect_container(container_name, None::<InspectContainerOptions>).await.is_err() {
            // Container not found, moving on
            warn!("Failed to download files from '{}' container", container_name);
            return Ok(HashMap::new());
        }

        let mut stream = self.docker.download_from_container(container_name, Some(options));

        let mut tar_bytes = Vec::new();

        while let Some(chunk) = stream.try_next().await? {
            tar_bytes.extend(chunk);
        }

        let cursor = Cursor::new(tar_bytes);
        let mut archive = Archive::new(cursor);

        let mut result = HashMap::new();

        for entry in archive.entries()? {
            let mut file = entry?;

            // Clone the path inside a string
            let path_str = file
                .path()?
                .to_string_lossy()
                .into_owned();

            if path_str.ends_with(".json") { // We only get JSON files
                let mut contents = String::new();
                use std::io::Read;
                file.read_to_string(&mut contents)?;

                if let Ok(json) = serde_json::from_str::<Value>(&contents) {
                    if let Some(uuid) = path_str.split('/').last() {
                        let uuid = uuid.replace(".json", "");
                        result.insert(uuid, json);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Récupère tous les fichiers avec l'extension donnée sous `remote_path` dans le container.
    /// Retourne une map { "<filename sans extension>" => bytes }.
    pub async fn fetch_files_by_extension(
        &self,
        container_name: &str,
        remote_path: &str,
        ext: &str,
    ) -> anyhow::Result<std::collections::HashMap<String, Vec<u8>>> {
        let options = DownloadFromContainerOptions {
            path: remote_path.to_string(),
        };

        if self.docker.inspect_container(container_name, None::<InspectContainerOptions>).await.is_err() {
            warn!("Failed to download files from '{}' container", container_name);
            return Ok(std::collections::HashMap::new());
        }

        let mut stream = self.docker.download_from_container(container_name, Some(options));
        let mut tar_bytes = Vec::new();

        while let Some(chunk) = stream.try_next().await? {
            tar_bytes.extend(chunk);
        }

        let cursor = Cursor::new(tar_bytes);
        let mut archive = Archive::new(cursor);
        let mut result = std::collections::HashMap::new();

        for entry in archive.entries()? {
            let mut file = entry?;
            let path_str = file.path()?.to_string_lossy().into_owned();

            if path_str.ends_with(&format!(".{}", ext)) {
                let mut contents = Vec::new();
                use std::io::Read;
                file.read_to_end(&mut contents)?;

                if let Some(fname) = path_str.split('/').last() {
                    let key = fname.replace(&format!(".{}", ext), "");
                    result.insert(key, contents);
                }
            }
        }

        Ok(result)
    }

}
