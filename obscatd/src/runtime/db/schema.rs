use rusqlite::{Connection, Result};

pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- 1. Tabel Deployment induk
        CREATE TABLE IF NOT EXISTS deployments (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        -- 2. Tabel Service (bagian dari deployment)
        CREATE TABLE IF NOT EXISTS services (
            id TEXT PRIMARY KEY,
            deployment_id TEXT NOT NULL,
            service_name TEXT NOT NULL,
            image TEXT NOT NULL,
            ip_address TEXT UNIQUE,
            port TEXT,
            FOREIGN KEY (deployment_id) REFERENCES deployments(id) ON DELETE CASCADE
        );

        -- 3. Tabel Container tunggal (dari obscat create)
        CREATE TABLE IF NOT EXISTS containers (
            id TEXT PRIMARY KEY,
            container_name TEXT NOT NULL,
            image TEXT NOT NULL,
            ip_address TEXT UNIQUE,
            port TEXT
        );

        -- Index untuk mempercepat pencarian saat undeploy & cleanup iptables
        CREATE INDEX IF NOT EXISTS idx_services_deployment ON services(deployment_id);
        ",
    )?;

    Ok(())
}
