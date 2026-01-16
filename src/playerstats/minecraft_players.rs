use crate::playerstats::{cobblemon_stats, DockerFetcher};
use serde_json::Value;
use std::collections::HashMap;
use colored::Colorize;
use log::{debug, error, info, trace, warn};
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

    info!(
        "{} {} {}",
        "Starting periodic playerstats fetch for".blue().bold(),
        "Minecraft".green().bold(),
        "players :".blue().bold()
    );
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

        let mut total_cobblemon_pokemon = 0;
        let mut total_cobblemon_trainer = 0;

        match cobblemon_stats::fetch_cobblemon_stats(server.id, container, world_name).await {
            Ok((pokemon, trainers)) => {
                total_cobblemon_pokemon = pokemon;
                total_cobblemon_trainer = trainers;
            }
            Err(e) => {
                    debug!(
                    "Failed to fetch cobblemon stats for server {}: {}",
                    server.nom.yellow().bold(),
                    e.to_string().yellow().bold()
                );
            }
        }

        trace!("Stats Map : {:?}", stats_map);

        let mut saved_count = 0; // Count number of playerstats saved
        let total_players = stats_map.len();

        // Filter and get specific values from the stats. Fallback to 0 if none found
        for (uuid, json) in stats_map {
            // Validate and format UUID
            let uuid = match helper::minecraft_account_formatter::check_and_format_minecraft_uuid(&uuid) {
                Ok(formatted_uuid) => formatted_uuid,
                Err(e) => {
                    warn!(
                "Invalid minecraft UUID : {} ; error: {}",
                uuid.yellow().bold(),
                e
            );
                    continue;
                }
            };

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
            let (
                tmps_jeux,
                nb_mort,
                nb_kills,
                nb_playerkill,
                nb_blocs_detr,
                nb_blocs_pose,
                dist_total,
                dist_pieds,
                dist_elytres,
                dist_vol,
                mob_killed,
                item_crafted,
                item_broken,
                achievement
            ) = extract_player_stats(&json);

            if db.add_or_update_playerstats(
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
            ).is_ok() {
                saved_count += 1; // Increment if save is successful
                info!("Minecraft playerstats added for player : {}", uuid.green().bold());
            } else {
                warn!("Failed to add/update player stats for uuid {}.", uuid.yellow().bold());
            }
        }

        // Send validation webhook
        let embed_color = if saved_count == 0 && total_players == 0 {
            "90c480".to_string() // Light green
        } else if saved_count == total_players {
            "126020".to_string() // Green
        } else {
            "601010".to_string() // Red
        };

        // Supertext
        let mut embed_supertext: String = format!("Enregistrement de {} joueurs sur {}", saved_count, total_players);

        if total_cobblemon_pokemon > 0 && total_cobblemon_trainer > 0 {
            embed_supertext.push_str(&format!(
                "\nEnregistrement de {} pokemon pour {} dresseur.",
                total_cobblemon_pokemon, total_cobblemon_trainer
            ));
        }

        if let Err(e) = helper::webhook_discord::send_discord_embed(
            "otternel",
            "",
            &format!("Playerstats fetch for {}", server.nom),
            "",
            &embed_supertext,
            embed_color.into(),
            &*server.image.unwrap(),
            "",
            "",
            &format!("{}", server.nom),
            Some(chrono::Utc::now().to_rfc3339())
        ) {
            error!("{e}");
        }
    }

    Ok(())
}

fn extract_player_stats(json: &Value) -> (
    i64, i32, i32, i32, i32, i32, i32, i32, i32, i32,
    Option<Value>, Option<Value>, Option<Value>, Option<Value>
) {
    let stats = json.get("stats").unwrap_or(&Value::Null);

    let mob_killed = stats.get("minecraft:killed").cloned();
    let item_crafted = stats.get("minecraft:crafted").cloned();
    let item_broken = stats.get("minecraft:broken").cloned();
    let achievement = stats.get("minecraft:achievements").cloned();

    let tmps_jeux = [
        "minecraft:play_one_minute",
        "minecraft:play_time"
    ].iter()
        .map(|k| stats.get("minecraft:custom").unwrap_or(&Value::Null).get(*k).and_then(|v| v.as_i64()).unwrap_or(0))
        .sum::<i64>() as i32;

    let nb_mort = stats.get("minecraft:custom")
        .and_then(|v| v.get("minecraft:deaths"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let nb_kills = stats.get("minecraft:custom")
        .and_then(|v| v.get("minecraft:mob_kills"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let nb_playerkill = stats.get("minecraft:custom")
        .and_then(|v| v.get("minecraft:player_kills"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let nb_blocs_detr = stats.get("minecraft:mined")
        .map(|custom| sum_stats_by_prefix(custom, "minecraft:"))
        .unwrap_or(0);

    let nb_blocs_pose = stats.get("minecraft:used")
        .map(|custom| sum_stats_by_prefix(custom, "minecraft:"))
        .unwrap_or(0);

    let dist_total = [
        "minecraft:climb_one_cm",
        "minecraft:crouch_one_cm",
        "minecraft:fall_one_cm",
        "minecraft:fly_one_cm",
        "minecraft:sprint_one_cm",
        "minecraft:swim_one_cm",
        "minecraft:walk_one_cm",
        "minecraft:walk_on_water_one_cm",
        "minecraft:walk_under_water_one_cm",
        "minecraft:boat_one_cm",
        "minecraft:aviate_one_cm",
        "minecraft:happy_ghast_one_cm",
        "minecraft:horse_one_cm",
        "minecraft:minecart_one_cm",
        "minecraft:pig_one_cm",
        "minecraft:strider_one_cm"
    ].iter()
        .map(|k| stats.get("minecraft:custom").unwrap_or(&Value::Null).get(*k).and_then(|v| v.as_i64()).unwrap_or(0))
        .sum::<i64>() as i32;

    let dist_pieds = [
        "minecraft:crouch_one_cm",
        "minecraft:sprint_one_cm",
        "minecraft:walk_one_cm",
        "minecraft:walk_on_water_one_cm",
        "minecraft:walk_under_water_one_cm"
    ].iter()
        .map(|k| stats.get("minecraft:custom").unwrap_or(&Value::Null).get(*k).and_then(|v| v.as_i64()).unwrap_or(0))
        .sum::<i64>() as i32;

    let dist_elytres = stats.get("minecraft:custom")
        .and_then(|v| v.get("minecraft:aviate_one_cm"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let dist_vol = stats.get("minecraft:custom")
        .and_then(|v| v.get("minecraft:fly_one_cm"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    (
        tmps_jeux as i64,
        nb_mort,
        nb_kills,
        nb_playerkill,
        nb_blocs_detr,
        nb_blocs_pose,
        dist_total,
        dist_pieds,
        dist_elytres,
        dist_vol,
        mob_killed,
        item_crafted,
        item_broken,
        achievement
    )
}

fn sum_stats_by_prefix(stats_obj: &Value, prefix: &str) -> i32 {
    if let Some(map) = stats_obj.as_object() {
        map.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(_, v)| {
                match v {
                    Value::Number(num) => num.as_i64().unwrap_or(0),
                    _ => 0
                }
            })
            .sum::<i64>() as i32
    } else {
        0
    }
}
