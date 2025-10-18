use mysql::{params, prelude::Queryable};
use crate::db::models::{Serveur};
use crate::db::models::{RconParams};

use super::repository_default::Database;

impl Database {
    // ===========================
    // serveurs
    // ===========================

    /// Fetch all serveurs from a game.
    pub fn get_all_server_by_game(
        &self,
        game: Box<str>,
    ) -> Result<Vec<Serveur>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let result: Vec<Serveur> = conn.exec_map(
            r#"SELECT id, nom, jeu, version, modpack, modpack_url, nom_monde, embed_color,
              contenaire, description, actif, global, type, image
       FROM serveurs
       WHERE jeu = :jeu"#,
            params! { "jeu" => game },
            |mut row: mysql::Row| {
                Serveur {
                    id: row.take("id").unwrap(),
                    nom: row.take("nom").unwrap(),
                    jeu: row.take("jeu").unwrap(),
                    version: row.take("version").unwrap(),
                    modpack: row.take("modpack"),
                    modpack_url: row.take("modpack_url"),
                    nom_monde: row.take("nom_monde"),
                    embed_color: row.take("embed_color"),
                    contenaire: row.take("contenaire"),
                    description: row.take("description").expect("REASON"),
                    actif: row.take("actif").unwrap(),
                    global: row.take("global").unwrap(),
                    r#type: row.take("type"),
                    image: row.take("image"),
                }
            },
        )?;

        Ok(result)
    }

    // ===========================
    // serveurs_actif
    // ===========================

    /// Fetch a `Serveur` using an active server (serveurs_actifs) id.
    /// Fetch a `Serveur` using an active server ID from the `serveurs_actifs` table.
    ///
    /// This function performs the following steps:
    /// 1. Looks up the `serveurs_id` associated with the given `active_id` in the `serveurs_actifs` table.
    /// 2. If a matching `serveurs_id` is found, retrieves the full `Serveur` record from the `serveurs` table.
    /// 3. Returns `Ok(Some(Serveur))` if found, or `Ok(None)` if no active server matches the given ID.
    ///
    /// # Arguments
    ///
    /// * `active_id` - The ID of the active server to look up.
    ///
    /// # Returns
    ///
    /// `Result<Option<Serveur>, mysql::Error>` - Returns the `Serveur` if found, otherwise `None`.
    pub fn get_server_by_active_server_id(
        &self,
        active_id: u64,
    ) -> Result<Option<Serveur>, mysql::Error> {
        let mut conn = self.get_conn()?;

        let serveurs_id_opt: Option<u64> = conn.exec_first(
            "SELECT serveurs_id FROM serveurs_actifs WHERE id = :id",
            params! { "id" => active_id },
        )?;

        let serveurs_id = match serveurs_id_opt {
            Some(id) => id,
            None => return Ok(None),
        };

        let result: Vec<Serveur> = conn.exec_map(
            r#"SELECT id, nom, jeu, version, modpack, modpack_url, nom_monde, embed_color,
                contenaire, description, actif, global, type, image
            FROM serveurs
            WHERE id = :id"#,
            params! { "id" => serveurs_id },
            |mut row: mysql::Row| {
                Serveur {
                    id: row.take("id").unwrap(),
                    nom: row.take("nom").unwrap(),
                    jeu: row.take("jeu").unwrap(),
                    version: row.take("version").unwrap(),
                    modpack: row.take("modpack"),
                    modpack_url: row.take("modpack_url"),
                    nom_monde: row.take("nom_monde"),
                    embed_color: row.take("embed_color"),
                    contenaire: row.take("contenaire"),
                    description: row.take("description").unwrap_or_default(),
                    actif: row.take("actif").unwrap(),
                    global: row.take("global").unwrap(),
                    r#type: row.take("type"),
                    image: row.take("image"),
                }
            },
        )?;

        Ok(result.into_iter().next())
    }

    /// Fetches RCON parameters for an active server by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The primary key (`id`) of the record in the `serveurs_actifs` table.
    ///
    /// # Returns
    ///
    /// `Result<Option<RconParams>, mysql::Error>` - Returns the RCON parameters if found, otherwise `None`.
    pub fn get_rcon_params_by_id(
        &self,
        id: u64,
    ) -> Result<Option<RconParams>, mysql::Error> {
        let mut conn = self.get_conn()?;

        // Use `exec_first` which is optimized for fetching a single row.
        // It returns an `Option` directly, simplifying the code.
        let result: Option<(String, String, String)> = conn.exec_first(
            "SELECT rcon_host, rcon_port, rcon_password FROM serveurs_actifs WHERE id = :id",
            params! { "id" => id },
        )?;

        // Map the resulting tuple into the RconParams struct.
        Ok(result.map(|(host, port, password)| RconParams {
            host,
            port,
            password,
        }))
    }

}
