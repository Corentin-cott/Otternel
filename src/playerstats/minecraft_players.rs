use crate::playerstats::DockerFetcher;
use serde_json::Value;
use std::collections::HashMap;
use colored::Colorize;
use log::{debug, info, warn};
use crate::helper;

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

pub async fn sync_mc_stats_to_db() -> anyhow::Result<()> {
    // Load configuration for DB pool before logging player connection
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return Ok(());
        }
    };

    // Get all Minecraft servers
    let minecraft_servers = db.get_all_server_by_game("minecraft".into())?;
    if minecraft_servers.is_empty() {
        warn!("No server could be found");
        return Ok(());
    }

    for server in minecraft_servers {
        info!("{} {}","Stating playerstats fetch for the server :".to_string().blue(), server.nom.green().bold() );

        // Fetch Minecraft stats from server files
        let container = server.contenaire.as_deref().ok_or_else(|| anyhow::anyhow!("none"))?;
        let world_name = server.nom_monde.as_deref().ok_or_else(|| anyhow::anyhow!("none"))?;

        let stats_map: HashMap<String, Value> = match fetch_mc_player_stats(container, world_name).await {
            Ok(map) => map,
            Err(e) => {
                warn!("Failed to fetch stats for server {}: {}", server.nom.yellow().bold(), e.to_string().yellow().bold());
                continue;
            }
        };

        // Filter and get specific values from the stats. Fallback to 0 if none found
        for (uuid, json) in stats_map {
            // We add the player in case they're not in the database already
            match db.add_player_if_not_exist("minecraft", uuid.clone()) {
                Ok(player_id) => {
                    debug!("Minecraft player with uuid : {} is in the database with id : {}", uuid.green().bold(), player_id.to_string().green().bold());
                }
                Err(e) => {
                    warn!("Could not check or add minecraft player with uuid : {} ; error: {}", uuid.yellow().bold(), e);
                    continue;
                }
            }

            // Now the stats
            let tmps_jeux = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:play_time"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let nb_mort = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:deaths"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let nb_kills = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:kill_entity"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let nb_playerkill = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:player_kills"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let mob_killed = json.get("minecraft:killed")
                .cloned();

            let nb_blocs_detr = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:mine_block"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let nb_blocs_pose = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:place_block"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let dist_total = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:walk_one_cm"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let dist_pieds = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:walk_on_water_one_cm"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let dist_elytres = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:fly_one_cm"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let dist_vol = json.get("minecraft:custom")
                .and_then(|v| v.get("minecraft:aviate_one_cm"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;

            let item_crafted = json.get("minecraft:crafted_items").cloned();
            let item_broken = json.get("minecraft:broken_items").cloned();
            let achievement = json.get("minecraft:achievements").cloned();

            // Appel de la méthode pour insérer ou mettre à jour en base
            if let Err(e) = db.add_or_update_playerstats(
                server.id.clone(),
                &uuid,
                tmps_jeux,
                nb_mort,
                nb_kills,
                nb_playerkill,
                mob_killed,
                nb_blocs_detr,
                nb_blocs_pose,
                dist_total,
                dist_pieds,
                dist_elytres,
                dist_vol,
                item_crafted,
                item_broken,
                achievement,
            ) {
                warn!("Failed to add/update player stats for uuid {}: {}", uuid.yellow().bold(), e);
                continue;
            } else {
                info!("Minecraft playerstats added for player : {}", uuid.green().bold());
            }
        }
    }

    Ok(())
}
