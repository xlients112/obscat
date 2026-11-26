use crate::runtime::db::con::get_conn;
use rusqlite::{Result, params};

pub fn save(id: &str, state: &str, container_name: &str, image: &str, ip: &str, port: &str) -> Result<()> {
    let conn = get_conn();
    conn.execute(
        "INSERT INTO containers (id, state, container_name, image, ip_address, port) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, state, container_name, image, ip, port],
    )?;
    Ok(())
}

pub fn get_network(id: &str) -> Result<Option<(String, String)>> {
    let conn = get_conn();
    let mut stmt = conn.prepare("SELECT ip_address, port FROM containers WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
        ))
    })?;

    match rows.next() {
        Some(res) => Ok(Some(res?)),
        None => Ok(None),
    }
}

pub fn delete(id: &str) -> Result<()> {
    let conn = get_conn();
    conn.execute("DELETE FROM containers WHERE id = ?1", params![id])?;
    Ok(())
}
