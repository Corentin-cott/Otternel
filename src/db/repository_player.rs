use colored::Colorize;
use log::{debug, warn};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::models::PlayerConnectionLog;
use crate::helper;

use super::repository_default::{Database, DbError};

impl Database {
    // ===========================
    // platforms (helper)
    // ===========================

    /// Resolves a `platforms` record id by its `name` (e.g. "minecraft").
    fn get_platform_id_by_name(&self, name: &str) -> Result<Option<String>, DbError> {
        let filter = format!("name='{}'", name);
        Ok(self
            .find_first("platforms", &filter)?
            .and_then(|v| v.get("id").and_then(|x| x.as_str()).map(String::from)))
    }

    // ===========================
    // players
    // ===========================

    /// Returns `true` if the player's `discord_user` relation is set.
    ///
    /// A missing player (or an empty relation) counts as not linked.
    pub fn is_account_linked_to_user(&self, player_id: &str) -> Result<bool, DbError> {
        match self.get_record("players", player_id) {
            Ok(v) => {
                let linked = v
                    .get("discord_user")
                    .and_then(|x| x.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
                Ok(linked)
            }
            Err(DbError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Updates a player's `last_connected_at` to the current UTC time.
    pub fn update_last_connection(&self, player_id: &str) -> Result<(), DbError> {
        let body = json!({ "last_connected_at": chrono::Utc::now().to_rfc3339() });
        self.update_record("players", player_id, &body)?;
        Ok(())
    }

    /// Finds a Minecraft player by `playername`, creating it (via the Mojang
    /// API) if it does not exist. Returns the player record id.
    pub fn add_and_get_minecraft_player_id(
        &self,
        username: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let filter = format!("playername='{}'", username);
        if let Some(p) = self.find_first("players", &filter)? {
            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }

        #[derive(Deserialize)]
        struct MojangResponse {
            id: String,
            name: String,
        }

        let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", username);
        let resp: MojangResponse = ureq::get(&url).call()?.into_json()?;

        let account_id =
            match helper::minecraft_account_formatter::check_and_format_minecraft_uuid(&resp.id) {
                Ok(formatted_uuid) => formatted_uuid,
                Err(e) => {
                    warn!(
                        "Invalid minecraft UUID : {} ; error: {}",
                        &resp.id.yellow().bold(),
                        e
                    );
                    return Err(Box::new(e));
                }
            };

        let platform_id = self
            .get_platform_id_by_name("minecraft")?
            .ok_or("No 'minecraft' platform found in PocketBase")?;

        let now = chrono::Utc::now().to_rfc3339();
        let body = json!({
            "platform": platform_id,
            "account_id": account_id,
            "playername": resp.name,
            "first_connected_at": now,
            "last_connected_at": now,
        });
        let created = self.create_record("players", &body)?;
        let id = created
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("created player record is missing an id")?;
        Ok(id.to_string())
    }

    /// Finds a player by `(platform, account_id)`, creating it (via the
    /// Minetools API to resolve the username) if it does not exist. Returns the
    /// player record id.
    pub fn add_player_if_not_exist(
        &self,
        game: &str,
        player_uuid: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let platform_id = self
            .get_platform_id_by_name(game)?
            .ok_or_else(|| format!("No '{}' platform found in PocketBase", game))?;

        let filter = format!("platform='{}' && account_id='{}'", platform_id, player_uuid);
        if let Some(p) = self.find_first("players", &filter)? {
            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }

        #[derive(Deserialize)]
        struct MojangResponse {
            id: String,
            name: String,
        }

        let url = format!("https://api.minetools.eu/uuid/{}", player_uuid);
        let resp: MojangResponse = match ureq::get(&url).call() {
            Ok(r) => match r.into_json() {
                Ok(json) => json,
                Err(_) => MojangResponse {
                    id: player_uuid.clone(),
                    name: "JoueurBedrock".into(),
                },
            },
            Err(ureq::Error::Status(code, _response)) => {
                // Probably a Bedrock or cracked player.
                if code == 204 || code == 404 {
                    MojangResponse {
                        id: player_uuid.clone(),
                        name: "JoueurBedrock".into(),
                    }
                } else {
                    return Err(Box::new(ureq::Error::Status(code, _response)));
                }
            }
            Err(e) => return Err(Box::new(e)),
        };

        let now = chrono::Utc::now().to_rfc3339();
        let body = json!({
            "platform": platform_id,
            "account_id": resp.id,
            "playername": resp.name,
            "first_connected_at": now,
            "last_connected_at": now,
        });
        let created = self.create_record("players", &body)?;
        let id = created
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("created player record is missing an id")?;

        debug!(
            "Added new player to database : {} with uuid : {}",
            resp.id.green().bold(),
            player_uuid.green().bold()
        );
        Ok(id.to_string())
    }

    // ===========================
    // player_connections_logs
    // ===========================

    /// Inserts a new player connection event.
    pub fn insert_player_connection_log(
        &self,
        log: &PlayerConnectionLog,
    ) -> Result<(), DbError> {
        let body = json!({
            "server": log.server,
            "player": log.player,
            "logged_at": log.logged_at,
        });
        self.create_record("player_connections_logs", &body)?;
        Ok(())
    }

    // ===========================
    // players_stats
    // ===========================

    /// Inserts or updates a player's stats for a server, keyed on
    /// `(server, account_id)`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_or_update_playerstats(
        &self,
        server_id: &str,
        account_id: &str,
        playtime: i64,
        deaths: i32,
        mob_kills: i32,
        player_kills: i32,
        mobs_killed: Option<Value>,
        blocks_mined: i32,
        blocks_placed: i32,
        total_distance: i32,
        distance_walked: i32,
        distance_elytra: i32,
        distance_fligth: i32,
        items_crafted: Option<Value>,
        items_broken: Option<Value>,
        achievements: Option<Value>,
    ) -> Result<(), DbError> {
        let body = json!({
            "server": server_id,
            "account_id": account_id,
            "playtime": playtime,
            "deaths": deaths,
            "mob_kills": mob_kills,
            "player_kills": player_kills,
            "mobs_killed": mobs_killed,
            "blocks_mined": blocks_mined,
            "blocks_placed": blocks_placed,
            "total_distance": total_distance,
            "distance_walked": distance_walked,
            "distance_elytra": distance_elytra,
            "distance_fligth": distance_fligth,
            "items_crafted": items_crafted,
            "items_broken": items_broken,
            "achievements": achievements,
        });

        let filter = format!("server='{}' && account_id='{}'", server_id, account_id);
        match self.find_first("players_stats", &filter)? {
            Some(existing) => {
                let id = existing.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                self.update_record("players_stats", id, &body)?;
            }
            None => {
                self.create_record("players_stats", &body)?;
            }
        }
        Ok(())
    }

    // ===========================
    // pokemon_players + captured_pokemons
    // ===========================

    /// Replaces a player's Pokemon party on a server.
    ///
    /// Each filled slot in `pkmn_data` (a slot is filled when its species is
    /// `Some`) becomes a fresh `captured_pokemons` record. The owning
    /// `pokemon_players` record (keyed on `server` + `player_uuid`) is then
    /// pointed at the new party, and the previously-linked `captured_pokemons`
    /// records are deleted to avoid orphans (they are keyed only by their own
    /// id, so they cannot be reused across syncs).
    pub fn insert_joueur_pokemon(
        &self,
        server_id: &str,
        player_uuid: &str,
        pkmn_data: &[(
            Option<&str>, // species
            Option<&str>, // form
            Option<&str>, // gender
            Option<&str>, // nickname
            Option<i32>,  // level
            Option<bool>, // shiny
            Option<&str>, // original owner uuid
            Option<&str>, // pokemon uuid
        ); 6],
    ) -> Result<(), DbError> {
        // 1. Create a captured_pokemons record per filled slot.
        let mut party_ids: Vec<String> = Vec::new();
        for (species, form, gender, nickname, level, shiny, do_uuid, pokemon_uuid) in
            pkmn_data.iter()
        {
            let Some(species) = species else { continue };
            let body = json!({
                "pokemon_name": species,
                "form": form,
                "gender": gender,
                "nickname": nickname,
                "level": level,
                "is_shiny": shiny,
                "pokemon_uuid": pokemon_uuid,
                "original_owner_uuid": do_uuid,
            });
            let created = self.create_record("captured_pokemons", &body)?;
            if let Some(id) = created.get("id").and_then(|v| v.as_str()) {
                party_ids.push(id.to_string());
            }
        }

        // 2. Upsert the owning pokemon_players record, remembering the old party.
        let filter = format!("server='{}' && player_uuid='{}'", server_id, player_uuid);
        let body = json!({
            "server": server_id,
            "player_uuid": player_uuid,
            "party": party_ids,
        });

        let old_party: Vec<String> = match self.find_first("pokemon_players", &filter)? {
            Some(existing) => {
                let id = existing
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let old = existing
                    .get("party")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                self.update_record("pokemon_players", &id, &body)?;
                old
            }
            None => {
                self.create_record("pokemon_players", &body)?;
                Vec::new()
            }
        };

        // 3. Delete the now-orphaned previous party members.
        for old_id in old_party {
            if let Err(e) = self.delete_record("captured_pokemons", &old_id) {
                warn!("Failed to delete orphaned captured_pokemon {}: {:?}", old_id, e);
            }
        }

        Ok(())
    }
}
