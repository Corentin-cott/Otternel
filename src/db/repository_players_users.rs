use mysql::{prelude::Queryable, params, PooledConn};
use mysql::Pool;

use crate::db::models::{
    Joueur,
    JoueurStats,
    JoueurConnectionLog,
    BadgeJoueur,
    UtilisateurDiscord,
};

pub struct Database {
    pool: Pool,
}

impl Database {
    /// Creates a MySQL connection pool for database operations.
    ///
    /// # Parameters
    /// - database_url: MySQL connection string.
    ///
    /// # Returns
    /// A new Database instance.
    ///
    /// # Errors
    /// Returns mysql::Error if the pool cannot be created.
    pub fn new(database_url: &str) -> Result<Self, mysql::Error> {
        let pool = Pool::new(database_url)?;
        Ok(Self { pool })
    }

    /// Retrieves a pooled MySQL connection for executing queries.
    ///
    /// # Errors
    /// Returns mysql::Error if a connection cannot be acquired from the pool.
    pub fn get_conn(&self) -> Result<PooledConn, mysql::Error> {
        self.pool.get_conn()
    }

    /// Checks database connectivity by performing a ping on a connection.
    ///
    /// # Returns
    /// Unit on successful ping.
    ///
    /// # Errors
    /// Returns mysql::Error if a connection cannot be acquired or ping fails.
    pub fn ping(&self) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.ping();
        Ok(())
    }

    // ===========================
    // Joueur
    // ===========================

    /// Fetches a Joueur by its unique identifier.
    ///
    /// # Parameters
    /// - id: Unique identifier of the player.
    ///
    /// # Returns
    /// Some(Joueur) if found, otherwise None.
    ///
    /// # Errors
    /// Returns mysql::Error if the query execution fails.
    pub fn get_joueur_by_id(&self, id: u64) -> Result<Option<Joueur>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let row: Option<mysql::Row> = conn.exec_first(
            "SELECT id, utilisateur_id, jeu, compte_id, premiere_co, derniere_co, playername
             FROM joueur
             WHERE id = :id",
            params! { "id" => id },
        )?;

        let joueur = row.map(|r| {
            let (id, utilisateur_id, jeu, compte_id, premiere_co, derniere_co, playername)
                = mysql::from_row(r);
            Joueur {
                id,
                utilisateur_id,
                jeu,
                compte_id,
                premiere_co,
                derniere_co,
                playername,
            }
        });

        Ok(joueur)
    }

    /// Inserts a new Joueur record and returns its generated identifier.
    ///
    /// # Parameters
    /// - j: Reference to the Joueur to insert.
    ///
    /// # Returns
    /// The last inserted identifier.
    ///
    /// # Errors
    /// Returns mysql::Error if the insert operation fails.
    pub fn insert_joueur(&self, j: &Joueur) -> Result<u64, mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "INSERT INTO joueur (utilisateur_id, jeu, compte_id, premiere_co, derniere_co, playername)
             VALUES (:utilisateur_id, :jeu, :compte_id, :premiere_co, :derniere_co, :playername)",
            params! {
                "utilisateur_id" => j.utilisateur_id,
                "jeu" => &j.jeu,
                "compte_id" => &j.compte_id,
                "premiere_co" => j.premiere_co,
                "derniere_co" => j.derniere_co,
                "playername" => &j.playername,
            },
        )?;
        Ok(conn.last_insert_id())
    }

    /// Updates an existing Joueur record by its identifier.
    ///
    /// # Parameters
    /// - j: Reference to the Joueur containing updated values (uses j.id).
    ///
    /// # Returns
    /// Unit on successful update.
    ///
    /// # Errors
    /// Returns mysql::Error if the update operation fails.
    pub fn update_joueur(&self, j: &Joueur) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "UPDATE joueur
             SET utilisateur_id = :utilisateur_id,
                 jeu = :jeu,
                 compte_id = :compte_id,
                 premiere_co = :premiere_co,
                 derniere_co = :derniere_co,
                 playername = :playername
             WHERE id = :id",
            params! {
                "id" => j.id,
                "utilisateur_id" => j.utilisateur_id,
                "jeu" => &j.jeu,
                "compte_id" => &j.compte_id,
                "premiere_co" => j.premiere_co,
                "derniere_co" => j.derniere_co,
                "playername" => &j.playername,
            },
        )?;
        Ok(())
    }

    // ===========================
    // JoueurStats
    // ===========================

    /// Fetches JoueurStats by its unique identifier.
    ///
    /// # Parameters
    /// - id: Unique identifier of the stats record.
    ///
    /// # Returns
    /// Some(JoueurStats) if found, otherwise None.
    ///
    /// # Errors
    /// Returns mysql::Error if the query execution fails.
    pub fn get_joueur_stats_by_id(&self, id: u64) -> Result<Option<JoueurStats>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let row: Option<mysql::Row> = conn.exec_first(
            "SELECT id, serveur_id, compte_id, tmps_jeux, nb_mort, nb_kills, nb_playerkill,
                    mob_killed, nb_blocs_detr, nb_blocs_pose, dist_total, dist_pieds, dist_elytres,
                    dist_vol, item_crafted, item_broken, achievement, derrn_enregistrement
             FROM joueur_stats
             WHERE id = :id",
            params! { "id" => id },
        )?;

        let stats = row.map(|r| {
            let (
                id, serveur_id, compte_id, tmps_jeux, nb_mort, nb_kills, nb_playerkill,
                mob_killed, nb_blocs_detr, nb_blocs_pose, dist_total, dist_pieds, dist_elytres,
                dist_vol, item_crafted, item_broken, achievement, derrn_enregistrement
            ) = mysql::from_row(r);

            JoueurStats {
                id,
                serveur_id,
                compte_id,
                tmps_jeux,
                nb_mort,
                nb_kills,
                nb_playerkill,
                mob_killed,
                nb_blocs_detr,
                nb_blocs_pose,
                dist_total,
                dist_pieds,
                dist_elytres,
                dist_vol,
                item_crafted,
                item_broken,
                achievement,
                derrn_enregistrement,
            }
        });

        Ok(stats)
    }

    /// Inserts or updates a JoueurStats record using ON DUPLICATE KEY UPDATE.
    ///
    /// # Parameters
    /// - s: Reference to the JoueurStats to upsert.
    ///
    /// # Returns
    /// Unit on successful upsert.
    ///
    /// # Errors
    /// Returns mysql::Error if the upsert operation fails.
    pub fn upsert_joueur_stats(&self, s: &JoueurStats) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "INSERT INTO joueur_stats (
                id, serveur_id, compte_id, tmps_jeux, nb_mort, nb_kills, nb_playerkill,
                mob_killed, nb_blocs_detr, nb_blocs_pose, dist_total, dist_pieds, dist_elytres,
                dist_vol, item_crafted, item_broken, achievement, derrn_enregistrement
             ) VALUES (
                :id, :serveur_id, :compte_id, :tmps_jeux, :nb_mort, :nb_kills, :nb_playerkill,
                :mob_killed, :nb_blocs_detr, :nb_blocs_pose, :dist_total, :dist_pieds, :dist_elytres,
                :dist_vol, :item_crafted, :item_broken, :achievement, :derrn_enregistrement
             )
             ON DUPLICATE KEY UPDATE
                serveur_id = VALUES(serveur_id),
                compte_id = VALUES(compte_id),
                tmps_jeux = VALUES(tmps_jeux),
                nb_mort = VALUES(nb_mort),
                nb_kills = VALUES(nb_kills),
                nb_playerkill = VALUES(nb_playerkill),
                mob_killed = VALUES(mob_killed),
                nb_blocs_detr = VALUES(nb_blocs_detr),
                nb_blocs_pose = VALUES(nb_blocs_pose),
                dist_total = VALUES(dist_total),
                dist_pieds = VALUES(dist_pieds),
                dist_elytres = VALUES(dist_elytres),
                dist_vol = VALUES(dist_vol),
                item_crafted = VALUES(item_crafted),
                item_broken = VALUES(item_broken),
                achievement = VALUES(achievement),
                derrn_enregistrement = VALUES(derrn_enregistrement)",
            params! {
                "id" => s.id,
                "serveur_id" => s.serveur_id,
                "compte_id" => s.compte_id,
                "tmps_jeux" => s.tmps_jeux,
                "nb_mort" => s.nb_mort,
                "nb_kills" => s.nb_kills,
                "nb_playerkill" => s.nb_playerkill,
                "mob_killed" => s.mob_killed,
                "nb_blocs_detr" => s.nb_blocs_detr,
                "nb_blocs_pose" => s.nb_blocs_pose,
                "dist_total" => s.dist_total,
                "dist_pieds" => s.dist_pieds,
                "dist_elytres" => s.dist_elytres,
                "dist_vol" => s.dist_vol,
                "item_crafted" => &s.item_crafted,
                "item_broken" => &s.item_broken,
                "achievement" => &s.achievement,
                "derrn_enregistrement" => s.derrn_enregistrement,
            },
        )?;
        Ok(())
    }

    // ===========================
    // JoueurConnectionLog
    // ===========================

    /// Inserts a connection log entry for a player and returns its identifier.
    ///
    /// # Parameters
    /// - log: Reference to the JoueurConnectionLog to insert.
    ///
    /// # Returns
    /// The last inserted identifier.
    ///
    /// # Errors
    /// Returns mysql::Error if the insert operation fails.
    pub fn insert_connection_log(&self, log: &JoueurConnectionLog) -> Result<u64, mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "INSERT INTO joueur_connection_log (serveur_id, joueur_id, date)
             VALUES (:serveur_id, :joueur_id, :date)",
            params! {
                "serveur_id" => log.serveur_id,
                "joueur_id" => log.joueur_id,
                "date" => log.date,
            },
        )?;
        Ok(conn.last_insert_id())
    }

    // ===========================
    // BadgeJoueur
    // ===========================

    /// Assigns a badge to a player and returns the new record identifier.
    ///
    /// # Parameters
    /// - badge: Reference to the BadgeJoueur to insert.
    ///
    /// # Returns
    /// The last inserted identifier.
    ///
    /// # Errors
    /// Returns mysql::Error if the insert operation fails.
    pub fn give_badge_to_joueur(&self, badge: &BadgeJoueur) -> Result<u64, mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "INSERT INTO badge_joueur (joueur_id, badge_id, date_recu)
             VALUES (:joueur_id, :badge_id, :date_recu)",
            params! {
                "joueur_id" => badge.joueur_id,
                "badge_id" => badge.badge_id,
                "date_recu" => badge.date_recu,
            },
        )?;
        Ok(conn.last_insert_id())
    }

    /// Lists all badges for a player ordered by reception date descending.
    ///
    /// # Parameters
    /// - joueur_id: Player identifier to filter badges.
    ///
    /// # Returns
    /// A vector of BadgeJoueur for the specified player.
    ///
    /// # Errors
    /// Returns mysql::Error if the query execution fails.
    pub fn list_badges_for_joueur(&self, joueur_id: u64) -> Result<Vec<BadgeJoueur>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let rows: Vec<mysql::Row> = conn.exec(
            "SELECT id, joueur_id, badge_id, date_recu
             FROM badge_joueur
             WHERE joueur_id = :joueur_id
             ORDER BY date_recu DESC",
            params! { "joueur_id" => joueur_id },
        )?;

        let badges = rows
            .into_iter()
            .map(|r| {
                let (id, joueur_id, badge_id, date_recu) = mysql::from_row(r);
                BadgeJoueur { id, joueur_id, badge_id, date_recu }
            })
            .collect();

        Ok(badges)
    }

    // ===========================
    // UtilisateurDiscord
    // ===========================

    /// Fetches a Discord user by its unique identifier.
    ///
    /// # Parameters
    /// - id: Unique identifier of the Discord user.
    ///
    /// # Returns
    /// Some(UtilisateurDiscord) if found, otherwise None.
    ///
    /// # Errors
    /// Returns mysql::Error if the query execution fails.
    pub fn get_utilisateur_discord_by_id(
        &self,
        id: u64,
    ) -> Result<Option<UtilisateurDiscord>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let row: Option<mysql::Row> = conn.exec_first(
            "SELECT id, discord_id, pseudo_discord, join_date_discord, first_activity,
                    last_activity, nb_message, tag_discord, avatar_url, vocal_time
             FROM utilisateur_discord
             WHERE id = :id",
            params! { "id" => id },
        )?;

        let user = row.map(|r| {
            let (
                id, discord_id, pseudo_discord, join_date_discord, first_activity,
                last_activity, nb_message, tag_discord, avatar_url, vocal_time
            ) = mysql::from_row(r);

            UtilisateurDiscord {
                id,
                discord_id,
                pseudo_discord,
                join_date_discord,
                first_activity,
                last_activity,
                nb_message,
                tag_discord,
                avatar_url,
                vocal_time,
            }
        });

        Ok(user)
    }

    /// Fetches a Discord user by their Discord snowflake identifier.
    ///
    /// # Parameters
    /// - discord_id: Discord snowflake (as string) to look up.
    ///
    /// # Returns
    /// Some(UtilisateurDiscord) if found, otherwise None.
    ///
    /// # Errors
    /// Returns mysql::Error if the query execution fails.
    pub fn get_utilisateur_discord_by_discord_id(
        &self,
        discord_id: &str,
    ) -> Result<Option<UtilisateurDiscord>, mysql::Error> {
        let mut conn = self.get_conn()?;
        let row: Option<mysql::Row> = conn.exec_first(
            "SELECT id, discord_id, pseudo_discord, join_date_discord, first_activity,
                    last_activity, nb_message, tag_discord, avatar_url, vocal_time
             FROM utilisateur_discord
             WHERE discord_id = :discord_id",
            params! { "discord_id" => discord_id },
        )?;

        let user = row.map(|r| {
            let (
                id, discord_id, pseudo_discord, join_date_discord, first_activity,
                last_activity, nb_message, tag_discord, avatar_url, vocal_time
            ) = mysql::from_row(r);

            UtilisateurDiscord {
                id,
                discord_id,
                pseudo_discord,
                join_date_discord,
                first_activity,
                last_activity,
                nb_message,
                tag_discord,
                avatar_url,
                vocal_time,
            }
        });

        Ok(user)
    }

    /// Inserts or updates a Discord user using ON DUPLICATE KEY UPDATE.
    ///
    /// # Parameters
    /// - u: Reference to the UtilisateurDiscord to upsert.
    ///
    /// # Returns
    /// Unit on successful upsert.
    ///
    /// # Errors
    /// Returns mysql::Error if the upsert operation fails.
    pub fn upsert_utilisateur_discord(&self, u: &UtilisateurDiscord) -> Result<(), mysql::Error> {
        let mut conn = self.get_conn()?;
        conn.exec_drop(
            "INSERT INTO utilisateur_discord (
                id, discord_id, pseudo_discord, join_date_discord, first_activity,
                last_activity, nb_message, tag_discord, avatar_url, vocal_time
             ) VALUES (
                :id, :discord_id, :pseudo_discord, :join_date_discord, :first_activity,
                :last_activity, :nb_message, :tag_discord, :avatar_url, :vocal_time
             )
             ON DUPLICATE KEY UPDATE
                discord_id = VALUES(discord_id),
                pseudo_discord = VALUES(pseudo_discord),
                join_date_discord = VALUES(join_date_discord),
                first_activity = VALUES(first_activity),
                last_activity = VALUES(last_activity),
                nb_message = VALUES(nb_message),
                tag_discord = VALUES(tag_discord),
                avatar_url = VALUES(avatar_url),
                vocal_time = VALUES(vocal_time)",
            params! {
                "id" => u.id,
                "discord_id" => &u.discord_id,
                "pseudo_discord" => &u.pseudo_discord,
                "join_date_discord" => u.join_date_discord,
                "first_activity" => u.first_activity,
                "last_activity" => u.last_activity,
                "nb_message" => u.nb_message,
                "tag_discord" => &u.tag_discord,
                "avatar_url" => &u.avatar_url,
                "vocal_time" => u.vocal_time,
            },
        )?;
        Ok(())
    }
}