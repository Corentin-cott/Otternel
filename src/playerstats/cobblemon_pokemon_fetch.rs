use anyhow::Result;
use flate2::read::GzDecoder;
use fastnbt::from_bytes;
use fastnbt::Value as NbtValue;
use log::{info, warn};
use std::io::Read;
// Assurez-vous que le chemin vers DockerFetcher est correct selon votre structure de dossiers
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
    _world_name: &str, // "_" car inutilisé pour l'instant
    fetcher: &DockerFetcher,
    remote_path: &str,
    db: &crate::Database // Vérifiez que "Database" est bien accessible à la racine
) -> Result<(usize, usize)> {
    
    let dat_files = fetcher
        .fetch_files_by_extension(container_name, remote_path, "dat")
        .await?;

    if dat_files.is_empty() {
        info!("No playerpartystore folder or .dat file found for '{}' (path: {})", container_name, remote_path);
        return Ok((0, 0));
    }

    let mut total_pokemon = 0usize;
    let mut total_joueurs = 0usize;

    for (file_key, bytes) in dat_files {
        let uuid = file_key;

        // Gestion propre de la décompression GZIP
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

                // On itère tant qu'on trouve des slots (Slot0, Slot1, etc.)
                for i in 0.. {
                    let slot_key = format!("Slot{}", i);
                    
                    if let Some(NbtValue::Compound(poke_nbt)) = compound.get(&slot_key) {
                        // --- PARSING SIMPLIFIÉ GRÂCE AUX HELPERS ---
                        let species = get_string(poke_nbt, "Species").unwrap_or_else(|| "unknown".to_string());
                        let form = get_string(poke_nbt, "FormId");
                        let gender = get_string(poke_nbt, "Gender");
                        let nickname = get_string(poke_nbt, "Nickname");
                        let level = get_int(poke_nbt, "Level");
                        let shiny = get_bool(poke_nbt, "Shiny");
                        
                        // Gestion des UUIDs (parfois appelés UUID, parfois PokemonUUID)
                        let pokemon_uuid = get_uuid(poke_nbt, "PokemonUUID").or_else(|| get_uuid(poke_nbt, "UUID"));
                        let do_uuid = get_uuid(poke_nbt, "PokemonOriginalTrainer");

                        team.push(Pokemon {
                            species, form, gender, nickname, level, shiny, do_uuid, pokemon_uuid
                        });
                    } else {
                        break; // Fin de l'équipe
                    }
                }

                if !team.is_empty() {
                    total_pokemon += team.len();
                    total_joueurs += 1;

                    // Préparation des données pour la BDD (Array fixe de 6 tuples)
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
                        warn!("Database insertion error for {}: {:?}", uuid, e);
                    }
                }
            }
            Ok(_) => warn!("Unexpected NBT root type for {}", uuid),
            Err(e) => warn!("NBT parse error for {}: {:?}", uuid, e),
        }
    }

    Ok((total_pokemon, total_joueurs))
}

// --- FONCTIONS UTILITAIRES (Helpers) ---

fn is_gzipped(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

fn decompress_gzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut gz = GzDecoder::new(bytes);
    let mut decomp = Vec::new();
    gz.read_to_end(&mut decomp)?;
    Ok(decomp)
}

// Récupère une String depuis le NBT
fn get_string(nbt: &fastnbt::Compound, key: &str) -> Option<String> {
    match nbt.get(key) {
        Some(NbtValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

// Récupère un Int (i32) depuis le NBT
fn get_int(nbt: &fastnbt::Compound, key: &str) -> Option<i32> {
    match nbt.get(key) {
        Some(NbtValue::Int(i)) => Some(*i),
        _ => None,
    }
}

// Récupère un Booléen (souvent stocké en Byte ou Int dans le NBT Minecraft)
fn get_bool(nbt: &fastnbt::Compound, key: &str) -> Option<bool> {
    match nbt.get(key) {
        Some(NbtValue::Byte(b)) => Some(*b != 0),
        Some(NbtValue::Int(i)) => Some(*i != 0),
        _ => None,
    }
}

// Récupère un UUID (String ou IntArray[4])
fn get_uuid(nbt: &fastnbt::Compound, key: &str) -> Option<String> {
    match nbt.get(key) {
        Some(NbtValue::String(s)) => Some(s.clone()),
        Some(NbtValue::IntArray(arr)) if arr.len() == 4 => {
            // Conversion standard Minecraft IntArray -> UUID String
            Some(format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                arr[0] as u32,
                (arr[1] >> 16) & 0xFFFF,
                arr[1] & 0xFFFF,
                (arr[2] >> 16) & 0xFFFF,
                (((arr[2] & 0xFFFF) as u64) << 32) | ((arr[3] as u32) as u64)
            ))
        }
        _ => None,
    }
}
