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

    /// Fetches all enabled, global servers that have a linked `active_servers`
    /// entry. Keyed on the `servers` record id, used for cross-server broadcasts.
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
            // The linked server must be present (expanded) to be usable.
            let Some(server_val) = item.get("expand").and_then(|e| e.get("server")) else {
                continue;
            };
            let server = self.parse_server(server_val)?;

            out.push(ActiveGlobalServer {
                server_id: server.id,
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

    /// Fetches a [`Server`] directly by its `servers` record id.
    ///
    /// Returns `Ok(None)` if no server matches `server_id`.
    pub fn get_server_by_id(&self, server_id: &str) -> Result<Option<Server>, DbError> {
        let filter = format!("id='{}'", server_id);
        let items = self.list_records(
            "servers",
            &[
                ("filter", filter.as_str()),
                ("expand", "platform"),
                ("perPage", "1"),
            ],
        )?;

        match items.into_iter().next() {
            Some(item) => Ok(Some(self.parse_server(&item)?)),
            None => Ok(None),
        }
    }

    /// Fetches RCON parameters for a server, resolved from the `active_servers`
    /// record whose `server` relation points at `server_id`.
    ///
    /// Returns `Ok(None)` if the server has no active entry.
    pub fn get_rcon_params_by_server_id(
        &self,
        server_id: &str,
    ) -> Result<Option<RconParams>, DbError> {
        let filter = format!("server='{}'", server_id);
        match self.find_first("active_servers", &filter)? {
            Some(v) => {
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
            None => Ok(None),
        }
    }
}
