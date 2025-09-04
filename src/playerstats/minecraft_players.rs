use crate::playerstats::DockerFetcher;
use serde_json::Value;
use std::collections::HashMap;

/// Récupère les stats des joueurs Minecraft dans un monde donné
/// # Parameters
/// - `container_name`: conteneur docker qui héberge le serveur MC
/// - `world_name`: nom du monde (ex: "world", "netherhub")
///
pub async fn fetch_mc_player_stats(
    container_name: &str,
    world_name: &str,
) -> anyhow::Result<HashMap<String, Value>> {
    let fetcher = DockerFetcher::new();

    // Path to stats folder
    let remote_path = format!("/server/{}/stats", world_name);

    let stats = fetcher.fetch_json_files(container_name, &remote_path).await?;
    Ok(stats)
}
