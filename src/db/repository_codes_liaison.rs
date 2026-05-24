use log::debug;
use serde_json::json;

use super::repository_default::{Database, DbError};

impl Database {
    // ===========================
    // linking_codes
    // ===========================

    /// Creates a new linking code for a player.
    ///
    /// # Arguments
    /// * `player_id` - The `players` record id requesting the link.
    /// * `code_valeur` - The generated code string.
    /// * `duree_minutes` - Validity duration in minutes; `expires_at` is
    ///   computed in Rust (PocketBase has no SQL `NOW() + INTERVAL`).
    pub fn save_linking_code(
        &self,
        player_id: &str,
        code_valeur: &str,
        duree_minutes: u32,
    ) -> Result<(), DbError> {
        debug!(
            "Saving linking code '{}' for player {} with duration {} minutes.",
            code_valeur, player_id, duree_minutes
        );

        let expires_at = (chrono::Utc::now() + chrono::Duration::minutes(duree_minutes as i64))
            .to_rfc3339();

        let body = json!({
            "player": player_id,
            "linking_code": code_valeur,
            "expires_at": expires_at,
        });
        self.create_record("linking_codes", &body)?;
        Ok(())
    }

    /// Returns `true` if the player has a linking code that has not yet expired.
    pub fn is_linking_code_active_for_player_id(&self, player_id: &str) -> Result<bool, DbError> {
        let now = chrono::Utc::now().to_rfc3339();
        let filter = format!("player='{}' && expires_at>'{}'", player_id, now);
        Ok(self.find_first("linking_codes", &filter)?.is_some())
    }
}
