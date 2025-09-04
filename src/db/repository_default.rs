use mysql::{PooledConn};
use mysql::Pool;

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

}