use crate::db::repository_default::Database;
use rcon::Connection;
use std::net::ToSocketAddrs;
use super::open_database::open_db_from_env;

use thiserror::Error;

/// Custom error type for RconHelper operations.
/// 
/// Uses `thiserror` for easy error definition and conversion.
#[derive(Debug, Error)]
pub enum RconHelperError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] mysql::Error),

    #[error("Could not find an active server with ID {0}")]
    ServerNotFound(u64),

    #[error("RCON error: {0}")]
        RconError(#[from] rcon::Error),

    #[error("Failed to initialize database connection")]
    DbInitError,
}

pub struct RconHelper {
    db: Database,
}

impl RconHelper {
    /// Creates a new instance of RconHelper with a database connection.
    pub fn new() -> Result<Self, RconHelperError> {
        let db = open_db_from_env().ok_or(RconHelperError::DbInitError)?;
        Ok(Self { db })
    }

    /// Executes an RCON command on a specific server.
    ///
    /// # Arguments
    /// * `active_server_id` - The ID from the `serveurs_actifs` table.
    /// * `command` - The command string to execute (no leading slash).
    ///
    /// # Returns
    /// The string response from the server.
    pub async fn execute_command(
        &self,
        active_server_id: u64,
        command: &str,
    ) -> Result<String, RconHelperError> {
        // Fetch RCON parameters from the database
        let rcon_params = self
            .db
            .get_rcon_params_by_id(active_server_id)?
            .ok_or(RconHelperError::ServerNotFound(active_server_id))?;

        // Establish a connection to the RCON server
        let addr = format!("{}:{}", rcon_params.host, rcon_params.port);
        let resolved_addr = addr.to_socket_addrs().unwrap().next().unwrap();
        
        let mut conn = Connection::builder()
            .connect(resolved_addr, &rcon_params.password)
            .await?;

        // Send the command and return the response
        let response = conn.cmd(command).await?;

        Ok(response)
    }
} 