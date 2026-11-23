use crate::runtime::db::con::get_conn;
use rusqlite::{params, Result};

pub fn save_deployment(id: &str, name: &str, version: &str) -> Result<()> {
    let conn = get_conn();
    conn.execute(
        "INSERT OR REPLACE INTO deployments (id, name, version, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![id, name, version],
    )?;
    Ok(())
}

pub fn save_service(
    id: &str,
    deployment_id: &str,
    service_name: &str,
    image: &str,
    ip_address: &str,
    port: &str,
) -> Result<()> {
    let conn = get_conn();
    conn.execute(
        "INSERT INTO services (id, deployment_id, service_name, image, ip_address, port)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, deployment_id, service_name, image, ip_address, port],
    )?;
    Ok(())
}

pub fn get_services_by_deployment(deployment_id: &str) -> Result<Vec<(String, String, String)>> {
    let conn = get_conn();
    let mut stmt = conn.prepare(
        "SELECT id, ip_address, port FROM services WHERE deployment_id = ?1",
    )?;

    let rows = stmt.query_map(params![deployment_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
        ))
    })?;

    rows.collect()
}

pub fn delete_deployment(deployment_id: &str) -> Result<()> {
    let conn = get_conn();
    conn.execute("DELETE FROM deployments WHERE id = ?1", params![deployment_id])?;
    Ok(())
}
