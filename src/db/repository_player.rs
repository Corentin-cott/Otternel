use mysql::{params, prelude::Queryable};
use serde::Deserialize;
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
}