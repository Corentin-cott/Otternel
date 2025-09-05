use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoueurConnectionLog {
    pub serveur_id: u64,
    pub joueur_id: u64,
    pub date: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Serveur {
    pub id: u64,
    pub nom: String,
    pub jeu: String,
    pub version: String,
    pub modpack: Option<String>,
    pub modpack_url: Option<String>,
    pub nom_monde: Option<String>,
    pub embed_color: Option<String>,
    pub contenaire: Option<String>,
    pub description: String,
    pub actif: bool,
    pub global: bool,
    pub r#type: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct JoueurStats {
    pub id: u64,
    pub serveur_id: u64,
    pub compte_id: String,
    pub tmps_jeux: i64,
    pub nb_mort: i32,
    pub nb_kills: i32,
    pub nb_playerkill: i32,
    pub mob_killed: Option<serde_json::Value>,
    pub nb_blocs_destr: i32,
    pub nb_blocs_pose: i32,
    pub dist_total: i32,
    pub dist_pieds: i32,
    pub dist_elytres: i32,
    pub dist_vol: i32,
    pub item_crafted: Option<serde_json::Value>,
    pub item_broken: Option<serde_json::Value>,
    pub achievement: Option<serde_json::Value>,
    pub dern_enregistr: NaiveDateTime,
}
