use colored::Colorize;
use log::debug;
use mysql::{params, prelude::Queryable};
use serde::Deserialize;
use serde_json::Value;
use crate::db::models::{JoueurConnectionLog};

use super::repository_default::Database;

impl Database {
    // ===========================
    // joueurs_connections_log
    // ===========================

    /// Insert a new `JoueurConnectionLog` into the database.
    ///
    /// # Arguments
    /// * `log` - A reference to a `JoueurConnectionLog` struct containing the data to insert.
    ///
    /// # Returns
    /// * `Ok(())` if the insertion succeeds.
    /// * `Err(mysql::Error)` if a MySQL error occurs.
    ///
    /// # Example
    /// ```rust
    /// let log = JoueurConnectionLog {
    ///     serveur_id: 42,
    ///     joueur_id: 123,
    ///     date: chrono::NaiveDateTime::from_timestamp(chrono::Utc::now().timestamp(), 0),
    /// };
    /// db.insert_joueur_connection_log(&log)?;
    /// ```
    pub fn insert_joueur_connection_log(
        &self,
        log: &JoueurConnectionLog,
    ) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;

        // Convert NaiveDateTime to string
        let date_str = log.date.format("%Y-%m-%d %H:%M:%S").to_string();

        conn.exec_drop(
            r#"
            INSERT INTO joueurs_connections_log (serveur_id, joueur_id, date)
            VALUES (:serveur_id, :joueur_id, :date)
            "#,
            params! {
                "serveur_id" => log.serveur_id,
                "joueur_id" => log.joueur_id,
                "date" => date_str
            },
        )?;

        Ok(())
    }

    pub fn add_and_get_minecraft_player_id(&self, username: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let mut conn = self.get_conn()?;

        // Check if player exists
        if let Some(row) = conn.exec_first::<(u64,), _, _>(
            "SELECT id FROM joueurs WHERE playername = :playername",
            params! { "playername" => username }
        )? {
            return Ok(row.0);
        }

        // Get current datetime
        let now = chrono::Utc::now().naive_utc();
        let date_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Fetch UUID from Mojang API
        #[derive(Deserialize)]
        struct MojangResponse {
            id: String,
            name: String,
        }

        let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", username);
        let resp: MojangResponse = ureq::get(&url)
            .call()?
            .into_json()?;

        // Insert new player into DB
        conn.exec_drop(
            r#"
        INSERT INTO joueurs (utilisateur_id, jeu, compte_id, playername, premiere_co, derniere_co)
        VALUES (:utilisateur_id, :jeu, :compte_id, :playername, :premiere_co, :derniere_co)
        "#,
            params! {
            "utilisateur_id" => Option::<u64>::None,
            "jeu" => "Minecraft",
            "compte_id" => &resp.id,
            "playername" => &resp.name,
            "premiere_co" => &date_str,
            "derniere_co" => &date_str,
        },
        )?;

        // Fetch the id of the newly inserted player
        let new_id = conn.exec_first::<(u64,), _, _>(
            "SELECT id FROM joueurs WHERE playername = :playername",
            params! { "playername" => username }
        )?.ok_or("Failed to retrieve new player id")?;

        Ok(new_id.0)
    }

    pub fn add_player_if_not_exist(
        &self,
        game: &str,
        player_uuid: String,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let mut conn = self.get_conn()?;

        if let Some(row) = conn.exec_first::<(u64,), _, _>(
            "SELECT id FROM joueurs WHERE jeu = :jeu AND compte_id = :compte_id",
            params! {
            "jeu" => game,
            "compte_id" => player_uuid.clone()
        },
        )? {
            return Ok(row.0);
        }

        let now = chrono::Utc::now().naive_utc();
        let date_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        // Fetch UUID from Mojang API
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
                // Probably bedrock or crack player
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

        conn.exec_drop(
            r#"
        INSERT INTO joueurs (utilisateur_id, jeu, compte_id, playername, premiere_co, derniere_co)
        VALUES (:utilisateur_id, :jeu, :compte_id, :playername, :premiere_co, :derniere_co)
        "#,
            params! {
            "utilisateur_id" => Option::<u64>::None,
            "jeu" => game,
            "compte_id" => &resp.id,
            "playername" => &resp.name,
            "premiere_co" => &date_str,
            "derniere_co" => &date_str,
        },
        )?;

        let new_id = conn.exec_first::<(u64,), _, _>(
            "SELECT id FROM joueurs WHERE jeu = :jeu AND compte_id = :compte_id",
            params! {
            "jeu" => game,
            "compte_id" => player_uuid.clone()
        },
        )?.ok_or("Failed to retrieve new player id")?;

        debug!("Added new player to database : {} with uuid : {}", &resp.id.to_string().green().bold(), player_uuid.green().bold());
        Ok(new_id.0)
    }

    pub fn add_or_update_playerstats(
        &self,
        serveur_id: u64,
        compte_id: &str,
        tmps_jeux: i64,
        nb_mort: i32,
        nb_kills: i32,
        nb_playerkill: i32,
        mob_killed: Option<serde_json::Value>,
        nb_blocs_detr: i32,
        nb_blocs_pose: i32,
        dist_total: i32,
        dist_pieds: i32,
        dist_elytres: i32,
        dist_vol: i32,
        item_crafted: Option<serde_json::Value>,
        item_broken: Option<serde_json::Value>,
        achievement: Option<serde_json::Value>,
    ) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;

        // Query
        conn.exec_drop(
            r#"
            INSERT INTO joueurs_stats (
                serveur_id, compte_id, tmps_jeux, nb_mort, nb_kills, nb_playerkill,
                mob_killed, nb_blocs_detr, nb_blocs_pose, dist_total, dist_pieds,
                dist_elytres, dist_vol, item_crafted, item_broken, achievement, dern_enregistrment
            ) VALUES (
                :serveur_id, :compte_id, :tmps_jeux, :nb_mort, :nb_kills, :nb_playerkill,
                :mob_killed, :nb_blocs_detr, :nb_blocs_pose, :dist_total, :dist_pieds,
                :dist_elytres, :dist_vol, :item_crafted, :item_broken, :achievement, NOW()
            )
            ON DUPLICATE KEY UPDATE
                tmps_jeux = VALUES(tmps_jeux),
                nb_mort = VALUES(nb_mort),
                nb_kills = VALUES(nb_kills),
                nb_playerkill = VALUES(nb_playerkill),
                mob_killed = VALUES(mob_killed),
                nb_blocs_detr = VALUES(nb_blocs_detr),
                nb_blocs_pose = VALUES(nb_blocs_pose),
                dist_total = VALUES(dist_total),
                dist_pieds = VALUES(dist_pieds),
                dist_elytres = VALUES(dist_elytres),
                dist_vol = VALUES(dist_vol),
                item_crafted = VALUES(item_crafted),
                item_broken = VALUES(item_broken),
                achievement = VALUES(achievement),
                dern_enregistrment = NOW()
            "#,
            params! {
                "serveur_id" => serveur_id,
                "compte_id" => compte_id,
                "tmps_jeux" => tmps_jeux,
                "nb_mort" => nb_mort,
                "nb_kills" => nb_kills,
                "nb_playerkill" => nb_playerkill,
                "mob_killed" => mob_killed.map(|v| v.to_string()),
                "nb_blocs_detr" => nb_blocs_detr,
                "nb_blocs_pose" => nb_blocs_pose,
                "dist_total" => dist_total,
                "dist_pieds" => dist_pieds,
                "dist_elytres" => dist_elytres,
                "dist_vol" => dist_vol,
                "item_crafted" => item_crafted.map(|v| v.to_string()),
                "item_broken" => item_broken.map(|v| v.to_string()),
                "achievement" => achievement.map(|v| v.to_string()),
            },
        )?;

        Ok(())
    }

}
