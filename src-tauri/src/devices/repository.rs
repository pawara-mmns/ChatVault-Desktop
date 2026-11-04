use super::models::{KnownDevice, Result, error};
use rusqlite::{Connection, params};
pub fn known(connection: &Connection) -> Result<Vec<KnownDevice>> {
    let mut statement = connection.prepare("SELECT id, display_name, category, last_connected_at, preferred FROM portable_device_notes ORDER BY preferred DESC, last_connected_at DESC LIMIT 100").map_err(|_| error("storage"))?;
    statement
        .query_map([], |row| {
            Ok(KnownDevice {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                last_connected_at: row.get::<_, i64>(3)? as u64,
                preferred: row.get(4)?,
            })
        })
        .map_err(|_| error("storage"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| error("storage"))
}
pub fn save(
    connection: &Connection,
    device: &super::models::Device,
    preferred: bool,
) -> Result<KnownDevice> {
    let id = device
        .saved_note_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let name = device
        .name
        .clone()
        .unwrap_or_else(|| "Portable device".into());
    if device.saved_note_id.is_none() {
        let count: i64 = connection
            .query_row("SELECT count(*) FROM portable_device_notes", [], |r| {
                r.get(0)
            })
            .map_err(|_| error("storage"))?;
        if count >= 100 {
            return Err(error("limit"));
        }
    }
    connection.execute("INSERT INTO portable_device_notes(id,display_name,category,last_connected_at,preferred) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,category=excluded.category,last_connected_at=excluded.last_connected_at,preferred=excluded.preferred", params![id, name, device.category, device.detected_at as i64, preferred]).map_err(|_| error("storage"))?;
    Ok(KnownDevice {
        id,
        name,
        category: device.category.clone(),
        last_connected_at: device.detected_at,
        preferred,
    })
}
