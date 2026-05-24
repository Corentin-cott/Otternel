use serde::{Deserialize, Serialize};

// PocketBase record ids are 15-char strings (not auto-increment integers), so
// every id and relation field is a `String`. Datetime-like fields are kept as
// RFC3339 strings: PocketBase accepts that for both `date` and `text` fields,
// which avoids coupling the models to a specific column type.

/// A server (PocketBase `servers` collection).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub r#type: Option<String>,
    /// Relation id pointing to the `platforms` collection.
    pub platform: String,
    /// Resolved platform name (e.g. "minecraft"), populated by the repository
    /// from `?expand=platform`. Not a real `servers` field.
    #[serde(skip)]
    pub platform_name: Option<String>,
    pub version: String,
    pub modpack: Option<String>,
    pub modpack_url: Option<String>,
    pub world_name: Option<String>,
    pub embed_color: Option<String>,
    pub container: Option<String>,
    #[serde(default)]
    pub description: String,
    pub is_enabled: bool,
    pub is_global: bool,
    /// Full file URL built by the repository. PocketBase stores only the
    /// filename, so the repository expands it to
    /// `{base}/api/files/servers/{id}/{filename}`.
    pub image: Option<String>,
}

/// A globally-broadcastable server: a `servers` record that is enabled and
/// global and has a linked `active_servers` entry (RCON creds). Used for
/// cross-server broadcasts.
#[derive(Debug, Clone, Default)]
pub struct ActiveGlobalServer {
    /// `servers` record id (the universal key; RCON is resolved from it).
    pub server_id: String,
    pub name: String,
    pub platform_name: Option<String>,
    pub version: String,
    pub embed_color: Option<String>,
    pub container: Option<String>,
    pub r#type: Option<String>,
}

/// RCON connection parameters from an `active_servers` record.
#[derive(Debug)]
pub struct RconParams {
    pub host: String,
    pub port: String,
    pub password: String,
}

/// A player connection event (PocketBase `player_connections_logs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConnectionLog {
    /// Relation id -> `servers`.
    pub server: String,
    /// Relation id -> `players`.
    pub player: String,
    /// RFC3339 UTC timestamp.
    pub logged_at: String,
}

/// Player statistics (PocketBase `players_stats`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    pub id: String,
    /// Relation id -> `servers`.
    pub server: String,
    pub account_id: String,
    pub playtime: i64,
    pub deaths: i32,
    pub mob_kills: i32,
    pub player_kills: i32,
    pub blocks_mined: i32,
    pub blocks_placed: i32,
    pub total_distance: i32,
    pub distance_walked: i32,
    pub distance_elytra: i32,
    /// Misspelled in the PocketBase schema (`distance_fligth`); kept as-is so
    /// serde matches the real field name.
    pub distance_fligth: i32,
    pub mobs_killed: Option<serde_json::Value>,
    pub items_crafted: Option<serde_json::Value>,
    pub items_broken: Option<serde_json::Value>,
    pub achievements: Option<serde_json::Value>,
}

/// A single captured Pokemon (PocketBase `captured_pokemons`). Each record is
/// distinct (keyed only by its own id, not by `pokemon_uuid`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapturedPokemon {
    pub id: String,
    pub pokemon_name: String,
    pub form: Option<String>,
    pub gender: Option<String>,
    pub nickname: Option<String>,
    pub level: Option<i32>,
    pub pokemon_uuid: Option<String>,
    pub original_owner_uuid: Option<String>,
    pub is_shiny: Option<bool>,
}

/// A player's Pokemon party on a server (PocketBase `pokemon_players`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PokemonPlayer {
    pub id: String,
    /// Relation id -> `servers`.
    pub server: String,
    pub player_uuid: String,
    /// Relation ids -> `captured_pokemons`.
    pub party: Vec<String>,
}

/// A player account (PocketBase `players` collection).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Player {
    pub id: String,
    pub account_id: String,
    pub playername: String,
    /// Relation id -> `discord_users`; empty/absent when not linked.
    pub discord_user: Option<String>,
    /// Relation id -> `platforms`. Required (Nonempty) — every player must
    /// have a platform.
    pub platform: String,
    /// RFC3339 UTC timestamps.
    pub first_connected_at: Option<String>,
    pub last_connected_at: Option<String>,
}

/// A Discord-account linking code (PocketBase `linking_codes` collection).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkingCode {
    pub id: String,
    /// Relation id -> `players`.
    pub player: String,
    pub linking_code: String,
    /// RFC3339 UTC timestamps.
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub used_at: Option<String>,
}
