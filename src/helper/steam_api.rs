use log::warn;
use serde::Deserialize;

/// Public Steam profile infos, from `GetPlayerSummaries`.
#[derive(Debug, Clone, Deserialize)]
pub struct SteamProfile {
    #[serde(rename = "avatarfull")]
    pub avatar_url: String,
    #[serde(rename = "profileurl")]
    pub profile_url: String,
}

#[derive(Deserialize)]
struct SummariesResponse {
    response: SummariesPlayers,
}

#[derive(Deserialize)]
struct SummariesPlayers {
    players: Vec<SteamProfile>,
}

/// Fetches a Steam player's public profile with the Steam Web API.
/// # Parameters
/// - `steam_id`: SteamID64 (`7656…`)
/// # Returns
/// - `None` if `STEAM_API_KEY` is not set, the API fails or the account doesn't exist
pub fn fetch_steam_profile(steam_id: &str) -> Option<SteamProfile> {
    let api_key = std::env::var("STEAM_API_KEY").ok().filter(|k| !k.is_empty())?;

    let resp: SummariesResponse = match ureq::get("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/")
        .query("key", &api_key)
        .query("steamids", steam_id)
        .call()
        .map_err(|e| e.to_string())
        .and_then(|r| r.into_json().map_err(|e| e.to_string()))
    {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Failed to fetch steam profile for {}: {}", steam_id, e);
            return None;
        }
    };

    resp.response.players.into_iter().next()
}
