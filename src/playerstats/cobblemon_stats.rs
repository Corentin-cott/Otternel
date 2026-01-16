use crate::playerstats::DockerFetcher;
use anyhow::Result;
use flate2::read::GzDecoder;
use fastnbt::from_bytes;
use fastnbt::Value as NbtValue;
use log::{info, warn};
use std::io::Read;
use crate::helper;

pub async fn fetch_cobblemon_stats(
    server_id: u64,
    container_name: &str,
    world_name: &str,
) -> Result<(usize, usize)> {
    // Load configuration for DB pool before logging player connection
    let db = match helper::open_database::open_db_from_env() {
        Some(db) => db,
        None => {
            warn!("Could not load DB configuration to resolve active server");
            return Ok((0, 0));
        }
    };

    // Create a Docker fetcher to fetch file stats
    let fetcher = DockerFetcher::new();
    let remote_path_playerpartystore = format!("/server/{}/pokemon/playerpartystore", world_name);

    let mut total_cobblemon_pokemon = 0;
    let mut total_cobblemon_trainer = 0;

    match cobblemon_stats::fetch_cobblemon_player_pokemons(server.id, container, world_name, fetcher, remote_path_playerpartystore).await {
        Ok((pokemon, trainers)) => {
            total_cobblemon_pokemon = pokemon;
            total_cobblemon_trainer = trainers;
        }
        Err(e) => {
            debug!(
                "Failed to fetch cobblemon player pokemon for server {}: {}",
                server.nom.yellow().bold(),
                e.to_string().yellow().bold()
            );
        }
    }

    Ok((total_cobblemon_pokemon, total_cobblemon_trainer))
}
