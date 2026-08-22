pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub db_path: String,
}
