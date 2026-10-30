use crate::helper;
use crate::db::models::PalworldPlayerStats;
use ::palworld_stats::{GuildInfo, Lang, PlayerStats, WorldStats, pal_name, pal_species_name, parse_world, passive_name};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use colored::Colorize;
use log::{debug, error, info, trace, warn};

/// Récupère et parse les sauvegardes d'un monde Palworld depuis le stockage local
/// # Parameters
/// - `server_id`: l'id du serveur (correspond au nom du dossier dans /stats/).
///   Le dossier du monde (`SaveGames/0/<worldId>`) y est monté directement :
///   `/stats/<server_id>/Level.sav` et `/stats/<server_id>/Players/`
/// # Returns
/// - Le `WorldStats` du monde (joueurs, guildes, avertissements)
pub async fn fetch_palworld_world_stats(
    server_id: &str,
) -> anyhow::Result<WorldStats> {
    let local_path = format!("/stats/{}", server_id);

    if !std::path::Path::new(&local_path).exists() {
        anyhow::bail!("Aucun dossier local pour ce serveur (Path: {})", local_path)
    }

    let level_bytes = tokio::fs::read(format!("{}/Level.sav", local_path)).await?;
    let player_files = fetch_local_player_files(&format!("{}/Players", local_path)).await?;

    debug!("{} player save files found for server {}", player_files.len(), server_id);

    // parse_world is synchronous and CPU heavy
    let world = tokio::task::spawn_blocking(move || parse_world(&level_bytes, player_files)).await??;

    Ok(world)
}

/// Lit tous les fichiers `.sav` du dossier `Players/`. Retourne `(nom du fichier, bytes)`
async fn fetch_local_player_files(path: &str) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut result = Vec::new();
    let mut entries = tokio::fs::read_dir(path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sav") {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let contents = tokio::fs::read(&path).await?;
                result.push((file_name.to_string(), contents));
            }
        }
    }

    Ok(result)
}

pub async fn sync_palworld_stats_to_db() -> anyhow::Result<()> {
    // Load configuration for DB pool before fetching stats
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return Ok(());
        }
    };

    // Get all Palworld servers
    let palworld_servers = db.get_all_server_by_game("palworld".into())?;
    if palworld_servers.is_empty() {
        warn!("No server could be found");
        return Ok(());
    }

    info!(
        "{} {} {}",
        "Starting periodic playerstats fetch for".blue().bold(),
        "Palworld".green().bold(),
        "players :".blue().bold()
    );

    for server in palworld_servers {
        info!("{} {} (ID: {})", "Stating playerstats fetch for:".to_string().blue(), server.name.green().bold(), server.id);

        // Fetch Palworld saves ONLY from local storage
        let world = match fetch_palworld_world_stats(&server.id).await {
            Ok(world) => world,
            Err(e) => {
                warn!(
                    "Failed to fetch palworld saves for server {}: {}",
                    server.name.yellow().bold(),
                    e.to_string().yellow().bold()
                );
                continue;
            }
        };

        process_and_save_stats(&db, &server, world).await?;
    }

    Ok(())
}

async fn process_and_save_stats(db: &crate::db::repository_default::Database, server: &crate::db::models::Server, world: WorldStats) -> anyhow::Result<()> {
    for w in &world.warnings {
        debug!("Palworld parse warning for server {}: {:?}", server.name, w);
    }

    trace!("World stats : {:?}", world);

    let mut saved_count = 0; // Count number of playerstats saved
    let total_players = world.players.len();

    for player in &world.players {
        let stats = extract_player_stats(&server.id, player, &world.guilds);

        // TODO: link the PlayerUid to a platform account (steam_… / xbox_…) to fill `player`
        if let Err(e) = db.add_or_update_palworld_stats(&stats) {
            warn!("Failed to add/update palworld stats for uid {} ; error: {:?}", stats.player_id.yellow().bold(), e);
            continue;
        }

        saved_count += 1; // Increment if save is successful
        info!("Palworld playerstats added for player : {}", stats.player_id.green().bold());
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

    if !world.guilds.is_empty() {
        embed_supertext.push_str(&format!("\n{} guildes trouvées.", world.guilds.len()));
    }

    if let Err(e) = helper::webhook_discord::send_discord_embed(
        "otternel",
        "",
        &format!("Playerstats fetch for {}", server.name),
        "",
        &embed_supertext,
        embed_color.into(),
        &*server.image.as_deref().unwrap_or(""),
        "",
        "",
        &format!("{}", server.name),
        Some(chrono::Utc::now().to_rfc3339())
    ) {
        error!("{e}");
    }

    Ok(())
}

/// Extrait les stats d'un joueur au format de la collection `palworld_stats`. Les champs
/// issus du fichier joueur sont `None` si ce fichier est absent ou illisible
fn extract_player_stats(server_id: &str, player: &PlayerStats, guilds: &[GuildInfo]) -> PalworldPlayerStats {
    let record = player.record.as_ref();
    let records = record.map(|r| &r.records);

    let guild_name = player.guild_id.as_ref()
        .and_then(|id| guilds.iter().find(|g| &g.id == id))
        .map(|g| g.name.clone());

    // Normal dungeons + fixed dungeons
    let dungeon_clear = records.and_then(|r| {
        match (r.normal_dungeon_clear, r.fixed_dungeon_clear) {
            (None, None) => None,
            (normal, fixed) => Some(normal.unwrap_or(0) + fixed.unwrap_or(0)),
        }
    });

    PalworldPlayerStats {
        player: None,
        server: server_id.to_string(),
        player_id: player.uid.to_string(),
        playername: player.name.clone(),
        guild_name,
        level: player.level,
        nb_catch_pal: records.and_then(|r| r.pal_capture_total),
        unlocked_technologies: record.and_then(|r| r.unlocked_technologies),
        completed_quests: record.and_then(|r| r.completed_quests),
        species_captured: records.and_then(|r| r.species_captured),
        tower_boss_defeated: records.and_then(|r| r.tower_boss_defeated),
        pal_butcher_total: records.and_then(|r| r.pal_butcher_total),
        dungeon_clear,
        oilrig_clear: records.and_then(|r| r.oilrig_clear),
        fishing_total: records.and_then(|r| r.fishing_total),
        craft_item_total: records.and_then(|r| r.craft_item_total),
        fast_travel_unlocked: records.and_then(|r| r.fast_travel_unlocked),
        game_cleared: records.and_then(|r| r.game_cleared).map(u8::from),
        pal_lucky: Some(player.pal_summary.lucky),
        // The party is only known from the player file
        pal_team: record.map(|_| pal_team_with_names(player)),
        capture_by_species: records.map(|r| counts_with_names(&r.pal_capture_by_species, |id| pal_name(id, Lang::Fr))),
        butcher_by_species: records.map(|r| counts_with_names(&r.butcher_by_species, |id| pal_name(id, Lang::Fr))),
        tower_boss_defeats: records.map(|r| counts_with_names(&r.tower_boss_defeats, tower_boss_name)),
        raid_boss_defeats: records.map(|r| counts_with_names(&r.raid_boss_defeats, raid_boss_name)),
        pal_summary: serde_json::to_value(&player.pal_summary).ok(),
    }
}

/// Pals of the party, each with its French names added: `name_fr` (variant title included),
/// `species_name_fr` and `passives_fr` (same order as `passives`, `null` when unknown)
fn pal_team_with_names(player: &PlayerStats) -> Value {
    let team = player.party().into_iter().filter_map(|pal| {
        let mut value = serde_json::to_value(pal).ok()?;
        if let Value::Object(fields) = &mut value {
            fields.insert("name_fr".into(), json!(pal_name(&pal.species, Lang::Fr)));
            fields.insert("species_name_fr".into(), json!(pal_species_name(&pal.species, Lang::Fr)));
            fields.insert(
                "passives_fr".into(),
                json!(pal.passives.iter().map(|p| passive_name(p, Lang::Fr)).collect::<Vec<_>>()),
            );
        }
        Some(value)
    }).collect();

    Value::Array(team)
}

/// Converts `{ id: count }` to `{ id: { "count": count, "name_fr": name } }`. The name is `null`
/// when unknown (e.g. `Human`, the single capture counter of every human)
fn counts_with_names(counts: &BTreeMap<String, i32>, name_fr: impl Fn(&str) -> Option<&'static str>) -> Value {
    let map: serde_json::Map<String, Value> = counts.iter()
        .map(|(id, count)| (id.clone(), json!({ "count": count, "name_fr": name_fr(id) })))
        .collect();

    Value::Object(map)
}

/// Tower of the save key (`GrassBoss_Hard`) -> character id of its boss in the names table.
/// The save only stores the tower, not the boss
const TOWER_BOSSES: &[(&str, &str)] = &[
    ("GrassBoss", "GYM_ElecPanda"),             // Zoe & Grizzbolt
    ("ForestBoss", "GYM_LilyQueen"),            // Lily & Lyleen
    ("ElectricBoss", "GYM_ThunderDragonMan"),   // Axel & Orserk
    ("SnowBoss", "GYM_Horus"),                  // Marcus & Faleris
    ("DesertBoss", "GYM_BlackGriffon"),         // Victor & Shadowbeak
    ("SakurajimaBoss", "GYM_MoonQueen"),        // Saya & Selyne
    ("VikingBoss", "GYM_SnowTigerBeastman"),    // Bjorn & Bastigor
    ("SorajimaBoss", "GYM_BlueSkyDragon"),      // Auri & Shaolong
    ("WorldTreeBoss", "GYM_WorldTreeDragon"),   // Zenara & Astralym
    ("KingWhaleBoss", "KingWhale"),             // Panthalus
];

/// French name of a tower boss key (`GrassBoss_Normal`, `GrassBoss_Hard`). Hard mode uses the
/// `_2` variant of the boss when it exists
fn tower_boss_name(id: &str) -> Option<&'static str> {
    let (tower, hard) = match id.rsplit_once('_') {
        Some((tower, "Hard")) => (tower, true),
        Some((tower, "Normal")) => (tower, false),
        _ => (id, false),
    };
    let boss = TOWER_BOSSES.iter().find(|(t, _)| *t == tower)?.1;

    hard.then(|| pal_name(&format!("{}_2", boss), Lang::Fr))
        .flatten()
        .or_else(|| pal_name(boss, Lang::Fr))
}

/// French name of a raid boss key (`PalSummon_NightLady`) from its `RAID_` entry
fn raid_boss_name(id: &str) -> Option<&'static str> {
    let boss = id.strip_prefix("PalSummon_").unwrap_or(id);

    // Some RAID_ entries only hold a placeholder (their own id): the name is then on the head
    [format!("RAID_{}", boss), format!("RAID_{}_Head", boss), boss.to_string()]
        .iter()
        .find_map(|candidate| pal_name(candidate, Lang::Fr).filter(|name| !name.eq_ignore_ascii_case(candidate)))
}
