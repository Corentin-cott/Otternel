use mysql::{params, prelude::Queryable};
use crate::db::models::{Serveur};

use super::repository_default::Database;

impl Database {
    // ===========================
    // serveurs_actif
    // ===========================

    /// Fetch a `Serveur` using an active server (serveurs_actifs) id.
    pub fn get_server_by_active_server_id(
        &self,
        active_id: u64,
    ) -> Result<Option<Serveur>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let result: Vec<Serveur> = conn.exec_map(
            r#"SELECT id, nom, jeu, version, modpack, modpack_url, nom_monde, embed_color,
              contenaire, description, actif, global, type, image
       FROM serveurs
       WHERE id = :id"#,
            params! { "id" => active_id },
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

        Ok(result.into_iter().next())
    }

}
