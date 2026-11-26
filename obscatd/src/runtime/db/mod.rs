use rusqlite::{Result, params};

use crate::runtime::db::con::get_conn;

pub mod con;
pub mod container;
pub mod deployment;
pub mod models;
pub mod schema;

pub struct ContainerListItem {
    pub id: String,
    pub display_name: String,
    pub image: String,
    pub ip_address: String,
    pub port: String,
}

pub struct ReconciliationItem {
    pub id: String,
    pub state: String,
}

// update state
pub fn update_container_state(container_id: &str, state: &str) -> Result<()> {
    let conn = get_conn();
    let rows_affected = conn.execute(
        "UPDATE containers SET state = ?1 WHERE id = ?2",
        params![state, container_id],
    )?;

    if rows_affected == 0 {
        conn.execute(
            "UPDATE services SET state = ?1 WHERE id = ?2",
            params![state, container_id],
        )?;
    }

    Ok(())
}

pub fn list_all_for_reconciliation() -> Result<Vec<ReconciliationItem>> {
    let conn = con::get_conn();
    let mut stmt = conn.prepare(
        "SELECT id, state FROM services
         UNION ALL
         SELECT id, state FROM containers",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ReconciliationItem {
            id: row.get(0)?,
            state: row.get(1)?,
        })
    })?;

    rows.collect()
}

pub fn list_all_items() -> Result<Vec<ContainerListItem>> {
    let conn = con::get_conn();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.service_name || ' (' || d.name || ')', s.image, s.ip_address, s.port
         FROM services s
         JOIN deployments d ON s.deployment_id = d.id
         UNION ALL
         SELECT id, container_name || ' (-)', image, ip_address, port
         FROM containers",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ContainerListItem {
            id: row.get(0)?,
            display_name: row.get(1)?,
            image: row.get(2)?,
            ip_address: row.get::<_, String>(3).unwrap_or_default(),
            port: row.get::<_, String>(4).unwrap_or_default(),
        })
    })?;

    rows.collect()
}

pub fn get_ip_and_port_by_id(id: &str) -> Result<Option<(String, String)>> {
    let conn = con::get_conn();
    let mut stmt = conn.prepare(
        "SELECT ip_address, port FROM services WHERE id = ?1
         UNION ALL
         SELECT ip_address, port FROM containers WHERE id = ?1
         LIMIT 1",
    )?;

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
