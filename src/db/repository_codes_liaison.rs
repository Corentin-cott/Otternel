use mysql::{params, prelude::Queryable, Row};
use chrono::NaiveDateTime;

use crate::db::models::{CodeLiaison, Serveur};

use super::repository_default::Database;

impl Database {    
    // ===========================
    // codes_liaison
    // ===========================

    /// Crée un nouveau code de liaison pour un joueur et l'insère en base de données.
    ///
    /// # Arguments
    ///
    /// * `joueur_id` - L'ID du joueur qui demande la liaison.
    /// * `code_valeur` - La chaîne de caractères unique générée pour le code.
    /// * `duree_minutes` - La durée de validité du code en minutes.
    ///
    /// # Returns
    ///
    /// `Result<(), mysql::Error>` - Retourne Ok(()) si l'insertion a réussi.
    pub fn creer_code_liaison(
        &self,
        joueur_id: u64,
        code_valeur: &str,
        duree_minutes: u32,
    ) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;
        
        conn.exec_drop(
            r#"INSERT INTO codes_liaison_compte (joueur_id, code_liaison, cree_le, expire_le)
               VALUES (:joueur_id, :code, NOW(), NOW() + INTERVAL :duree MINUTE)"#,
            params! {
                "joueur_id" => joueur_id,
                "code" => code_valeur,
                "duree" => duree_minutes,
            },
        )?;

        Ok(())
    }
}