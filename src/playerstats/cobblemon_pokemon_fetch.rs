use anyhow::Result;
use flate2::read::GzDecoder;
use fastnbt::from_bytes;
use fastnbt::Value as NbtValue;
use log::{info, warn};
use std::io::Read;
use std::collections::HashMap;
use crate::playerstats::DockerFetcher;

#[derive(Debug)]
struct Pokemon {
    species: String,
    form: Option<String>,
    gender: Option<String>,
    nickname: Option<String>,
    level: Option<i32>,
    shiny: Option<bool>,
    do_uuid: Option<String>,
    pokemon_uuid: Option<String>,
}

pub async fn fetch_cobblemon_player_pokemons(
    server_id: u64,
    container_name: &str,
    _world_name: &str, // Unused for now
    fetcher: &DockerFetcher,
    remote_path: &str,
    db: &crate::db::repository_default::Database,
) -> Result<(usize, usize)> {

    let dat_files = fetcher
        .fetch_files_by_extension(container_name, remote_path, "dat")
        .await?;

    if dat_files.is_empty() {
        info!(
            "No playerpartystore folder or .dat file found for '{}' (path: {})",
            container_name,
            remote_path
        );
        return Ok((0, 0));
    }

    let mut total_pokemon = 0usize;
    let mut total_players = 0usize;

    for (file_key, bytes) in dat_files {
        let uuid = file_key;

        // Proper GZIP decompression handling
        let decompressed_bytes = if is_gzipped(&bytes) {
            match decompress_gzip(&bytes) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Decompression error for {}: {:?}", uuid, e);
                    continue;
                }
            }
        } else {
            bytes
        };

        match from_bytes::<NbtValue>(&decompressed_bytes) {
            Ok(NbtValue::Compound(compound)) => {
                let mut team: Vec<Pokemon> = Vec::new();

                // Iterate while Slot0, Slot1, Slot2... exist
                for i in 0.. {
                    let slot_key = format!("Slot{}", i);

                    if let Some(NbtValue::Compound(poke_nbt)) = compound.get(&slot_key) {

                        // --- Simplified parsing using helper functions ---
                        let species = get_string(poke_nbt, "Species")
                            .unwrap_or_else(|| "unknown".to_string());
                        let form = get_string(poke_nbt, "FormId");
                        let gender = get_string(poke_nbt, "Gender");
                        let nickname = get_string(poke_nbt, "Nickname");
                        let level = get_int(poke_nbt, "Level");
                        let shiny = get_bool(poke_nbt, "Shiny");

                        // UUID fields may have different names depending on context
                        let pokemon_uuid = get_uuid(poke_nbt, "PokemonUUID")
                            .or_else(|| get_uuid(poke_nbt, "UUID"));
                        let do_uuid = get_uuid(poke_nbt, "PokemonOriginalTrainer");

                        team.push(Pokemon {
                            species,
                            form,
                            gender,
                            nickname,
                            level,
                            shiny,
                            do_uuid,
                            pokemon_uuid,
                        });
                    } else {
                        break; // Stop when no more slots are found
                    }
                }

                if !team.is_empty() {
                    total_pokemon += team.len();
                    total_players += 1;

                    // Prepare fixed-size array (max team size = 6)
                    let mut pkmn_data =
                        [(None, None, None, None, None, None, None, None); 6];

                    for (i, poke) in team.iter().enumerate().take(6) {
                        pkmn_data[i] = (
                            Some(poke.species.as_str()),
                            poke.form.as_deref(),
                            poke.gender.as_deref(),
                            poke.nickname.as_deref(),
                            poke.level,
                            poke.shiny,
                            poke.do_uuid.as_deref(),
                            poke.pokemon_uuid.as_deref(),
                        );
                    }

                    if let Err(e) =
                        db.insert_joueur_pokemon(server_id, &uuid, &pkmn_data)
                    {
                        warn!("Database insertion error for {}: {:?}", uuid, e);
                    }
                }
            }
            Ok(_) => warn!("Unexpected NBT root type for {}", uuid),
            Err(e) => warn!("NBT parse error for {}: {:?}", uuid, e),
        }
    }

    Ok((total_pokemon, total_players))
}

// TODO : Move these helper methods in helper crate

/// Checks if the byte slice starts with a GZIP header
fn is_gzipped(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

/// Decompresses GZIP-compressed data into a Vec<u8>
fn decompress_gzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut gz = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    gz.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Extracts a String from an NBT compound
fn get_string(
    nbt: &HashMap<String, NbtValue>,
    key: &str,
) -> Option<String> {
    match nbt.get(key) {
        Some(NbtValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Extracts an i32 from an NBT compound
fn get_int(
    nbt: &HashMap<String, NbtValue>,
    key: &str,
) -> Option<i32> {
    match nbt.get(key) {
        Some(NbtValue::Int(i)) => Some(*i),
        _ => None,
    }
}

/// Extracts a boolean value (commonly stored as Byte or Int in Minecraft NBT)
fn get_bool(
    nbt: &HashMap<String, NbtValue>,
    key: &str,
) -> Option<bool> {
    match nbt.get(key) {
        Some(NbtValue::Byte(b)) => Some(*b != 0),
        Some(NbtValue::Int(i)) => Some(*i != 0),
        _ => None,
    }
}

/// Extracts a UUID from either a String or an IntArray[4]
fn get_uuid(
    nbt: &HashMap<String, NbtValue>,
    key: &str,
) -> Option<String> {
    match nbt.get(key) {
        Some(NbtValue::String(s)) => Some(s.clone()),
        Some(NbtValue::IntArray(arr)) if arr.len() == 4 => {
            // Standard Minecraft IntArray[4] -> UUID conversion
            Some(format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                arr[0] as u32,
                (arr[1] >> 16) & 0xFFFF,
                arr[1] & 0xFFFF,
                (arr[2] >> 16) & 0xFFFF,
                (((arr[2] & 0xFFFF) as u64) << 32)
                    | ((arr[3] as u32) as u64)
            ))
        }
        _ => None,
    }
}