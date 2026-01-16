use colored::Colorize;
use log::debug;
use mysql::{params, prelude::Queryable};
use serde::Deserialize;
use crate::db::models::{JoueurConnectionLog};
use crate::helper;
use log::{warn};

use super::repository_default::Database;

impl Database {

    // ===========================
    // joueurs
    // ===========================

    /// Vérifie si un compte joueur est déjà lié à un utilisateur.
    ///
    /// La liaison est déterminée par la présence d'une valeur non-NULL
    /// dans la colonne `utilisateur_id` de la table `joueurs`.
    ///
    /// # Arguments
    ///
    /// * `joueur_id` - L'ID du joueur à vérifier.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` si le champ `utilisateur_id` n'est pas NULL.
    /// - `Ok(false)` si le champ `utilisateur_id` est NULL ou si le joueur n'existe pas.
    /// - `Err(mysql::Error)` en cas d'erreur de communication avec la base de données.
    pub fn is_account_linked_to_user(&self, joueur_id: u64) -> Result<bool, mysql::Error> {
        let mut conn = self.get_conn()?;

        let count: Option<u64> = conn.exec_first(
            "SELECT COUNT(utilisateur_id) FROM joueurs WHERE id = :id",
            params! { "id" => joueur_id },
        )?;

        // Si le joueur n'existe pas, `count` sera `None`. `unwrap_or(0)` transforme `None` en 0
        Ok(count.unwrap_or(0) > 0)
    }

    /// Met à jour la date de dernière connexion d'un joueur via son ID interne.
    ///
    /// # Arguments
    /// * `joueur_id` - L'ID unique du joueur dans la base de données (table `joueurs`).
    pub fn update_last_connection(&self, joueur_id: u64) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;

        // On récupère la date actuelle UTC formatée comme dans tes autres méthodes
        let now = chrono::Utc::now().naive_utc();
        let date_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        conn.exec_drop(
            r"UPDATE joueurs SET derniere_co = :date WHERE id = :id",
            params! {
                "date" => date_str,
                "id" => joueur_id,
            },
        )?;

        Ok(())
    }

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

        // Checking player uuid (account_id)
        let player_account_id = match helper::minecraft_account_formatter::check_and_format_minecraft_uuid(&resp.id) {
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

        // Insert new player into DB
        conn.exec_drop(
            r#"
        INSERT INTO joueurs (utilisateur_id, jeu, compte_id, playername, premiere_co, derniere_co)
        VALUES (:utilisateur_id, :jeu, :compte_id, :playername, :premiere_co, :derniere_co)
        "#,
            params! {
            "utilisateur_id" => Option::<u64>::None,
            "jeu" => "Minecraft",
            "compte_id" => player_account_id,
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

    pub fn insert_joueur_pokemon(
        &self,
        serveur_id: u64,
        joueur_uuid: &str,
        pkmn_data: &[(
            Option<&str>,  // pkmn
            Option<&str>,  // form
            Option<&str>,  // gender
            Option<&str>,  // nickname
            Option<i32>,   // level
            Option<bool>,  // shiny
            Option<&str>,  // do_uuid
            Option<&str>,  // pokemon_uuid
        ); 6],
    ) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;

        conn.exec_drop(
            r#"
            INSERT INTO joueurs_pokemon (
                serveur_id, joueur_uuid,
                pkmn1, pkmn1_form, pkmn1_gender, pkmn1_nickname, pkmn1_level, pkmn1_shiny, pkmn1_do_uuid, pkmn1_uuid,
                pkmn2, pkmn2_form, pkmn2_gender, pkmn2_nickname, pkmn2_level, pkmn2_shiny, pkmn2_do_uuid, pkmn2_uuid,
                pkmn3, pkmn3_form, pkmn3_gender, pkmn3_nickname, pkmn3_level, pkmn3_shiny, pkmn3_do_uuid, pkmn3_uuid,
                pkmn4, pkmn4_form, pkmn4_gender, pkmn4_nickname, pkmn4_level, pkmn4_shiny, pkmn4_do_uuid, pkmn4_uuid,
                pkmn5, pkmn5_form, pkmn5_gender, pkmn5_nickname, pkmn5_level, pkmn5_shiny, pkmn5_do_uuid, pkmn5_uuid,
                pkmn6, pkmn6_form, pkmn6_gender, pkmn6_nickname, pkmn6_level, pkmn6_shiny, pkmn6_do_uuid, pkmn6_uuid
            ) VALUES (
                :serveur_id, :joueur_uuid,
                :pkmn1, :pkmn1_form, :pkmn1_gender, :pkmn1_nickname, :pkmn1_level, :pkmn1_shiny, :pkmn1_do_uuid, :pkmn1_uuid,
                :pkmn2, :pkmn2_form, :pkmn2_gender, :pkmn2_nickname, :pkmn2_level, :pkmn2_shiny, :pkmn2_do_uuid, :pkmn2_uuid,
                :pkmn3, :pkmn3_form, :pkmn3_gender, :pkmn3_nickname, :pkmn3_level, :pkmn3_shiny, :pkmn3_do_uuid, :pkmn3_uuid,
                :pkmn4, :pkmn4_form, :pkmn4_gender, :pkmn4_nickname, :pkmn4_level, :pkmn4_shiny, :pkmn4_do_uuid, :pkmn4_uuid,
                :pkmn5, :pkmn5_form, :pkmn5_gender, :pkmn5_nickname, :pkmn5_level, :pkmn5_shiny, :pkmn5_do_uuid, :pkmn5_uuid,
                :pkmn6, :pkmn6_form, :pkmn6_gender, :pkmn6_nickname, :pkmn6_level, :pkmn6_shiny, :pkmn6_do_uuid, :pkmn6_uuid
            )
            ON DUPLICATE KEY UPDATE
                pkmn1 = VALUES(pkmn1), pkmn1_form = VALUES(pkmn1_form), pkmn1_gender = VALUES(pkmn1_gender),
                pkmn1_nickname = VALUES(pkmn1_nickname), pkmn1_level = VALUES(pkmn1_level),
                pkmn1_shiny = VALUES(pkmn1_shiny), pkmn1_do_uuid = VALUES(pkmn1_do_uuid), pkmn1_uuid = VALUES(pkmn1_uuid),
                pkmn2 = VALUES(pkmn2), pkmn2_form = VALUES(pkmn2_form), pkmn2_gender = VALUES(pkmn2_gender),
                pkmn2_nickname = VALUES(pkmn2_nickname), pkmn2_level = VALUES(pkmn2_level),
                pkmn2_shiny = VALUES(pkmn2_shiny), pkmn2_do_uuid = VALUES(pkmn2_do_uuid), pkmn2_uuid = VALUES(pkmn2_uuid),
                pkmn3 = VALUES(pkmn3), pkmn3_form = VALUES(pkmn3_form), pkmn3_gender = VALUES(pkmn3_gender),
                pkmn3_nickname = VALUES(pkmn3_nickname), pkmn3_level = VALUES(pkmn3_level),
                pkmn3_shiny = VALUES(pkmn3_shiny), pkmn3_do_uuid = VALUES(pkmn3_do_uuid), pkmn3_uuid = VALUES(pkmn3_uuid),
                pkmn4 = VALUES(pkmn4), pkmn4_form = VALUES(pkmn4_form), pkmn4_gender = VALUES(pkmn4_gender),
                pkmn4_nickname = VALUES(pkmn4_nickname), pkmn4_level = VALUES(pkmn4_level),
                pkmn4_shiny = VALUES(pkmn4_shiny), pkmn4_do_uuid = VALUES(pkmn4_do_uuid), pkmn4_uuid = VALUES(pkmn4_uuid),
                pkmn5 = VALUES(pkmn5), pkmn5_form = VALUES(pkmn5_form), pkmn5_gender = VALUES(pkmn5_gender),
                pkmn5_nickname = VALUES(pkmn5_nickname), pkmn5_level = VALUES(pkmn5_level),
                pkmn5_shiny = VALUES(pkmn5_shiny), pkmn5_do_uuid = VALUES(pkmn5_do_uuid), pkmn5_uuid = VALUES(pkmn5_uuid),
                pkmn6 = VALUES(pkmn6), pkmn6_form = VALUES(pkmn6_form), pkmn6_gender = VALUES(pkmn6_gender),
                pkmn6_nickname = VALUES(pkmn6_nickname), pkmn6_level = VALUES(pkmn6_level),
                pkmn6_shiny = VALUES(pkmn6_shiny), pkmn6_do_uuid = VALUES(pkmn6_do_uuid), pkmn6_uuid = VALUES(pkmn6_uuid)
            "#,
            params! {
                "serveur_id" => serveur_id,
                "joueur_uuid" => joueur_uuid,
                "pkmn1" => pkmn_data[0].0, "pkmn1_form" => pkmn_data[0].1, "pkmn1_gender" => pkmn_data[0].2,
                "pkmn1_nickname" => pkmn_data[0].3, "pkmn1_level" => pkmn_data[0].4, "pkmn1_shiny" => pkmn_data[0].5,
                "pkmn1_do_uuid" => pkmn_data[0].6, "pkmn1_uuid" => pkmn_data[0].7,
                "pkmn2" => pkmn_data[1].0, "pkmn2_form" => pkmn_data[1].1, "pkmn2_gender" => pkmn_data[1].2,
                "pkmn2_nickname" => pkmn_data[1].3, "pkmn2_level" => pkmn_data[1].4, "pkmn2_shiny" => pkmn_data[1].5,
                "pkmn2_do_uuid" => pkmn_data[1].6, "pkmn2_uuid" => pkmn_data[1].7,
                "pkmn3" => pkmn_data[2].0, "pkmn3_form" => pkmn_data[2].1, "pkmn3_gender" => pkmn_data[2].2,
                "pkmn3_nickname" => pkmn_data[2].3, "pkmn3_level" => pkmn_data[2].4, "pkmn3_shiny" => pkmn_data[2].5,
                "pkmn3_do_uuid" => pkmn_data[2].6, "pkmn3_uuid" => pkmn_data[2].7,
                "pkmn4" => pkmn_data[3].0, "pkmn4_form" => pkmn_data[3].1, "pkmn4_gender" => pkmn_data[3].2,
                "pkmn4_nickname" => pkmn_data[3].3, "pkmn4_level" => pkmn_data[3].4, "pkmn4_shiny" => pkmn_data[3].5,
                "pkmn4_do_uuid" => pkmn_data[3].6, "pkmn4_uuid" => pkmn_data[3].7,
                "pkmn5" => pkmn_data[4].0, "pkmn5_form" => pkmn_data[4].1, "pkmn5_gender" => pkmn_data[4].2,
                "pkmn5_nickname" => pkmn_data[4].3, "pkmn5_level" => pkmn_data[4].4, "pkmn5_shiny" => pkmn_data[4].5,
                "pkmn5_do_uuid" => pkmn_data[4].6, "pkmn5_uuid" => pkmn_data[4].7,
                "pkmn6" => pkmn_data[5].0, "pkmn6_form" => pkmn_data[5].1, "pkmn6_gender" => pkmn_data[5].2,
                "pkmn6_nickname" => pkmn_data[5].3, "pkmn6_level" => pkmn_data[5].4, "pkmn6_shiny" => pkmn_data[5].5,
                "pkmn6_do_uuid" => pkmn_data[5].6, "pkmn6_uuid" => pkmn_data[5].7,
            },
        )?;

        Ok(())
    }

}
