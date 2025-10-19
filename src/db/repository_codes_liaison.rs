use log::{error, warn, info, debug};
use mysql::{params, prelude::Queryable};

use super::repository_default::Database;

impl Database {
    // ===========================
    // codes_liaison
    // ===========================

    /// Creates a new linking code for a player and inserts it into the database.
    ///
    /// # Arguments
    ///
    /// * `joueur_id` - The ID of the player requesting the link.
    /// * `code_valeur` - The unique string generated for the code.
    /// * `duree_minutes` - The code's validity duration in minutes.
    ///
    /// # Returns
    ///
    /// `Result<(), mysql::Error>` - Returns Ok(()) if the insertion was successful.
    pub fn save_linking_code(
        &self,
        joueur_id: u64,
        code_valeur: &str,
        duree_minutes: u32,
    ) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;

        debug!(
            "Saving linking code '{}' for player ID {} with duration {} minutes.",
            code_valeur, joueur_id, duree_minutes
        );
        
        conn.exec_drop(
            r#"INSERT INTO codes_liaison (joueur_id, code_liaison, cree_le, expire_le)
               VALUES (:joueur_id, :code, NOW(), NOW() + INTERVAL :duree MINUTE)"#,
            params! {
                "joueur_id" => joueur_id,
                "code" => code_valeur,
                "duree" => duree_minutes,
            },
        )?;

        Ok(())
    }

    /// Checks if an active linking code exists for a given player.
    ///
    /// # Arguments
    ///
    /// * `joueur_id` - The ID of the player to check for an active code.
    ///
    /// # Returns
    ///
    /// `Result<bool, mysql::Error>` - Returns `Ok(true)` if an active code exists,
    /// `Ok(false)` otherwise, or an error in case of a database issue.
    pub fn is_linking_code_active_for_player_id(&self, joueur_id: u64) -> Result<bool, mysql::Error> {
        let mut conn = self.get_conn()?;
        
        let code_existe: Option<u8> = conn.exec_first(
            r#"SELECT EXISTS(
                SELECT 1
                FROM codes_liaison
                WHERE joueur_id = :joueur_id AND expire_le > NOW()
            )"#,
            params! { "joueur_id" => joueur_id },
        )?;

        Ok(code_existe.map_or(false, |val| val == 1))
    }
}