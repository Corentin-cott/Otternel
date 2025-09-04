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