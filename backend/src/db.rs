#[cfg(not(any(feature = "db-sqlite", feature = "db-postgres")))]
compile_error!("Either the `db-sqlite` or `db-postgres` feature must be enabled.");

#[cfg(all(feature = "db-sqlite", feature = "db-postgres"))]
compile_error!("Only one of `db-sqlite` or `db-postgres` can be enabled.");

#[cfg(feature = "db-postgres")]
pub use sqlx::postgres::{PgPool as DbPool, PgPoolOptions as DbPoolOptions, Postgres as Db};

#[cfg(feature = "db-sqlite")]
pub use sqlx::sqlite::{Sqlite as Db, SqlitePool as DbPool, SqlitePoolOptions as DbPoolOptions};
