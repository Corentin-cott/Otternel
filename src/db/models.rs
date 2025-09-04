use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joueur {
    pub id: u64,
    pub utilisateur_id: u64,
    pub jeu: String,
    pub compte_id: String,
    pub premiere_co: Option<NaiveDateTime>,
    pub derniere_co: Option<NaiveDateTime>,
    pub playername: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoueurStats {
    pub id: u64,
    pub serveur_id: u64,
    pub compte_id: u64,
    pub tmps_jeux: u64,
    pub nb_mort: u64,
    pub nb_kills: u64,
    pub nb_playerkill: u64,
    pub mob_killed: u64,
    pub nb_blocs_detr: u64,
    pub nb_blocs_pose: u64,
    pub dist_total: u64,
    pub dist_pieds: u64,
    pub dist_elytres: u64,
    pub dist_vol: u64,
    pub item_crafted: Option<String>,
    pub item_broken: Option<String>,
    pub achievement: Option<String>,
    pub derrn_enregistrement: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoueurConnectionLog {
    pub id: u64,
    pub serveur_id: u64,
    pub joueur_id: u64,
    pub date: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeJoueur {
    pub id: u64,
    pub joueur_id: u64,
    pub badge_id: u64,
    pub date_recu: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilisateurDiscord {
    pub id: u64,
    pub discord_id: String,
    pub pseudo_discord: String,
    pub join_date_discord: NaiveDateTime,
    pub first_activity: Option<NaiveDateTime>,
    pub last_activity: Option<NaiveDateTime>,
    pub nb_message: u64,
    pub tag_discord: String,
    pub avatar_url: Option<String>,
    pub vocal_time: u64,
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
pub struct ServeurActif {
    pub id: u64,
    pub serveurs_id: u64,
    pub host: String,
    pub rcon_host: Option<String>,
    pub rcon_port: String,
    pub rcon_password: Option<String>,
}
