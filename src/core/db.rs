//! # Database – Connection Pool & Migration Runner
//!
//! Khởi tạo PostgreSQL connection pool và tự động chạy migrations khi startup.
//! Migrations được đọc từ thư mục `migrations/` và chạy theo thứ tự file name.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::Path;
use tracing;

/// Tạo connection pool tới PostgreSQL.
///
/// Pool config tối ưu cho production:
/// - max_connections: 20 (đủ cho hầu hết workloads, tránh quá tải DB)
/// - min_connections: 2 (giữ connections warm)
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    tracing::info!("✅ Database connection pool established");
    Ok(pool)
}

/// Tự động chạy tất cả migration files theo thứ tự.
///
/// Đọc folder `migrations/`, sort theo filename (000_, 001_, ...),
/// và thực thi từng file SQL. Sử dụng bảng `_migrations` để track
/// file nào đã chạy, tránh chạy lại.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Tạo bảng tracking migrations nếu chưa có
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            id          SERIAL PRIMARY KEY,
            filename    VARCHAR(255) NOT NULL UNIQUE,
            applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Đọc migration files
    let migrations_dir = Path::new("migrations");
    if !migrations_dir.exists() {
        tracing::warn!("⚠️ No migrations directory found, skipping migrations");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(migrations_dir)
        .map_err(|e| sqlx::Error::Configuration(format!("Failed to read migrations dir: {e}").into()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "sql")
                .unwrap_or(false)
        })
        .collect();

    // Sort theo filename để đảm bảo thứ tự: 000_ → 001_ → 002_ ...
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let filename = entry.file_name().to_string_lossy().to_string();

        // Kiểm tra đã chạy chưa
        let already_applied = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM _migrations WHERE filename = $1)",
        )
        .bind(&filename)
        .fetch_one(pool)
        .await?;

        if already_applied {
            tracing::debug!("⏭️ Migration already applied: {}", filename);
            continue;
        }

        // Đọc và thực thi SQL
        let sql = std::fs::read_to_string(entry.path()).map_err(|e| {
            sqlx::Error::Configuration(format!("Failed to read migration {filename}: {e}").into())
        })?;

        tracing::info!("🔄 Applying migration: {}", filename);

        sqlx::raw_sql(&sql).execute(pool).await.map_err(|e| {
            tracing::error!("❌ Migration failed: {} – {}", filename, e);
            e
        })?;

        // Ghi nhận đã apply
        sqlx::query("INSERT INTO _migrations (filename) VALUES ($1)")
            .bind(&filename)
            .execute(pool)
            .await?;

        tracing::info!("✅ Migration applied: {}", filename);
    }

    Ok(())
}
