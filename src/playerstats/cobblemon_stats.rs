use crate::playerstats::DockerFetcher;
use anyhow::Result;
use flate2::read::GzDecoder;
use fastnbt::from_bytes;
use fastnbt::Value as NbtValue;
use log::{info, warn};
use std::io::Read;
use crate::helper;

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

    let fetcher = DockerFetcher::new();
    let remote_path = format!("/server/{}/pokemon/playerpartystore", world_name);

    let dat_files = fetcher
        .fetch_files_by_extension(container_name, &remote_path, "dat")
        .await?;

    if dat_files.is_empty() {
        info!(
            "No playerpartystore folder or .dat file found for '{}' (path: {})",
            container_name, remote_path
        );
        return Ok((0, 0));
    }

    let mut total_pokemon = 0usize;
    let mut total_joueurs = 0usize;

    for (file_key, bytes) in dat_files {
        let uuid = file_key; // Filename = joueur UUID

        // Décompression GZIP si nécessaire
        let decompressed_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            let mut gz = GzDecoder::new(&bytes[..]);
            let mut decomp = Vec::new();
            if let Err(e) = gz.read_to_end(&mut decomp) {
                warn!("Decompression error {}: {:?}", uuid, e);
                continue;
            }
            decomp
        } else {
            bytes
        };

        match from_bytes::<NbtValue>(&decompressed_bytes) {
            Ok(nbt) => {
                if let NbtValue::Compound(compound) = nbt {
                    let mut team: Vec<Pokemon> = Vec::new();

                    for i in 0.. {
                        let slot_key = format!("Slot{}", i);
                        if let Some(NbtValue::Compound(poke)) = compound.get(&slot_key) {
                            let species = match poke.get("Species") {
                                Some(NbtValue::String(s)) => s.clone(),
                                _ => "unknown".to_string(),
                            };
                            let form = match poke.get("FormId") {
                                Some(NbtValue::String(s)) => Some(s.clone()),
                                _ => None,
                            };
                            let gender = match poke.get("Gender") {
                                Some(NbtValue::String(s)) => Some(s.clone()),
                                _ => None,
                            };
                            let nickname = match poke.get("Nickname") {
                                Some(NbtValue::String(s)) => Some(s.clone()),
                                _ => None,
                            };
                            let level = match poke.get("Level") {
                                Some(NbtValue::Int(n)) => Some(*n),
                                _ => None,
                            };
                            let shiny = match poke.get("Shiny") {
                                Some(NbtValue::Byte(b)) => Some(*b != 0),
                                Some(NbtValue::Int(i)) => Some(*i != 0),
                                _ => None,
                            };
                            let pokemon_uuid = match poke.get("PokemonUUID").or_else(|| poke.get("UUID")) {
                                Some(NbtValue::String(s)) => Some(s.clone()),
                                Some(NbtValue::IntArray(arr)) => {
                                    if arr.len() == 4 {
                                        Some(format!(
                                            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                                            arr[0],
                                            (arr[1] >> 16) & 0xFFFF,
                                            arr[1] & 0xFFFF,
                                            (arr[2] >> 16) & 0xFFFF,
                                            (((arr[2] & 0xFFFF) as u64) << 32) | ((arr[3] as u32) as u64)
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };
                            let do_uuid = match poke.get("PokemonOriginalTrainer") {
                                Some(NbtValue::String(s)) => Some(s.clone()),
                                Some(NbtValue::IntArray(arr)) => {
                                    if arr.len() == 4 {
                                        Some(format!(
                                            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                                            arr[0],
                                            (arr[1] >> 16) & 0xFFFF,
                                            arr[1] & 0xFFFF,
                                            (arr[2] >> 16) & 0xFFFF,
                                            (((arr[2] & 0xFFFF) as u64) << 32) | ((arr[3] as u32) as u64)
                                        ))
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };

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
                            break;
                        }
                    }

                    total_pokemon += team.len();
                    total_joueurs += 1;

                    let mut pkmn_data = [(None, None, None, None, None, None, None, None); 6];

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

                    if let Err(e) = db.insert_joueur_pokemon(server_id, &uuid, &pkmn_data) {
                        warn!("Erreur insertion joueurs_pokemon pour {}: {:?}", uuid, e);
                    }
                } else {
                    println!("Unexpected NBT root type for {}", uuid);
                }
            }
            Err(e) => warn!("Can't parse NBT for {}: {:?}", uuid, e),
        }
    }

    Ok((total_pokemon, total_joueurs))
}
