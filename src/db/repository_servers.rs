use serde_json::Value;

use crate::db::models::{ActiveGlobalServer, RconParams, Server};

use super::repository_default::{Database, DbError};

impl Database {
    // ===========================
    // Record -> model helpers
    // ===========================

    /// Parses a PocketBase `servers` record into a [`Server`].
    ///
    /// Expects the record to optionally carry `expand.platform` (to resolve
    /// `platform_name`). The raw `image` filename is rewritten into a full file
    /// URL; an empty filename becomes `None`.
    fn parse_server(&self, v: &Value) -> Result<Server, DbError> {
        let mut server: Server =
            serde_json::from_value(v.clone()).map_err(|e| DbError::Decode(e.to_string()))?;

        // Resolve the human-readable platform name from the expanded relation.
        server.platform_name = v
            .get("expand")
            .and_then(|e| e.get("platform"))
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        // PocketBase stores only the filename; expand it to a full file URL.
        server.image = server
            .image
            .filter(|f| !f.is_empty())
            .map(|f| format!("{}/api/files/servers/{}/{}", self.base_url(), server.id, f));

        Ok(server)
    }

    // ===========================
    // servers
    // ===========================

    /// Fetches all servers for a given game (matched on `platform.name`).
    pub fn get_all_server_by_game(&self, game: Box<str>) -> Result<Vec<Server>, DbError> {
        let filter = format!("platform.name='{}'", game);
        let items = self.list_records(
            "servers",
            &[
                ("filter", filter.as_str()),
                ("expand", "platform"),
                ("perPage", "200"),
            ],
        )?;

        items.iter().map(|v| self.parse_server(v)).collect()
    }

    // ===========================
    // active_servers
    // ===========================

    /// Fetches all active servers whose linked server is enabled and global.
    ///
    /// Returns the `active_servers` record id alongside the linked server's
    /// display info, used for cross-server broadcasts.
    pub fn get_all_active_global_servers(&self) -> Result<Vec<ActiveGlobalServer>, DbError> {
        let items = self.list_records(
            "active_servers",
            &[
                ("filter", "server.is_enabled=true && server.is_global=true"),
                ("expand", "server.platform"),
                ("perPage", "200"),
            ],
        )?;

        let mut out = Vec::with_capacity(items.len());
        for item in &items {
            let active_id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            // The linked server must be present (expanded) to be usable.
            let Some(server_val) = item.get("expand").and_then(|e| e.get("server")) else {
                continue;
            };
            let server = self.parse_server(server_val)?;

            out.push(ActiveGlobalServer {
                active_id,
                name: server.name,
                platform_name: server.platform_name,
                version: server.version,
                embed_color: server.embed_color,
                container: server.container,
                r#type: server.r#type,
            });
        }
        Ok(out)
    }

    /// Fetches the [`Server`] linked to an `active_servers` record.
    ///
    /// Returns `Ok(None)` if no active server matches `active_id` or it has no
    /// linked server.
    pub fn get_server_by_active_server_id(
        &self,
        active_id: &str,
    ) -> Result<Option<Server>, DbError> {
        let filter = format!("id='{}'", active_id);
        let items = self.list_records(
            "active_servers",
            &[
                ("filter", filter.as_str()),
                ("expand", "server.platform"),
                ("perPage", "1"),
            ],
        )?;

        let Some(item) = items.into_iter().next() else {
            return Ok(None);
        };
        let Some(server_val) = item.get("expand").and_then(|e| e.get("server")) else {
            return Ok(None);
        };
        Ok(Some(self.parse_server(server_val)?))
    }

    /// Fetches RCON parameters from an `active_servers` record by its id.
    ///
    /// Returns `Ok(None)` if the record does not exist.
    pub fn get_rcon_params_by_id(&self, active_id: &str) -> Result<Option<RconParams>, DbError> {
        match self.get_record("active_servers", active_id) {
            Ok(v) => {
                let field = |key: &str| {
                    v.get(key)
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                Ok(Some(RconParams {
                    host: field("rcon_host"),
                    port: field("rcon_port"),
                    password: field("rcon_password"),
                }))
            }
            Err(DbError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
