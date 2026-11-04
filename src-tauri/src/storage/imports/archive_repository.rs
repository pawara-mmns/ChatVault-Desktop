use super::{
    Result, error,
    media::AttachmentLink,
    models::{ArchivePage, ArchiveSummary, DateOrder, Diagnostic, ParsedChat},
    service::Session,
};
use rusqlite::{Connection, OptionalExtension, Row, params};

pub fn duplicate(connection: &Connection, hash: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT id FROM archives WHERE original_sha256=?1",
            [hash],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| error("import_database", "Could not check duplicate imports."))
}
fn row_summary(row: &Row<'_>) -> rusqlite::Result<ArchiveSummary> {
    Ok(ArchiveSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        original_filename: row.get(2)?,
        source_format: row.get(3)?,
        imported_at: row.get(4)?,
        message_count: row.get(5)?,
        media_count: row.get(6)?,
        missing_media_count: row.get(7)?,
        total_bytes: row.get(8)?,
        import_status: row.get(9)?,
        original_sha256: row.get(10)?,
        date_order: row.get(11)?,
        text_encoding: row.get(12)?,
        diagnostics: Vec::new(),
    })
}
const COLUMNS: &str = "id,title,original_filename,source_format,created_at,message_count,media_count,missing_media_count,total_bytes,import_status,original_sha256,date_order,text_encoding";
pub fn summary(connection: &Connection, id: &str) -> Result<ArchiveSummary> {
    if id.len() > 128 {
        return Err(error(
            "archive_missing",
            "The requested archive does not exist.",
        ));
    }
    let mut summary = connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM archives WHERE id=?1"),
            [id],
            row_summary,
        )
        .optional()
        .map_err(|_| error("import_database", "Could not read archive metadata."))?
        .ok_or_else(|| error("archive_missing", "The requested archive does not exist."))?;
    let mut statement=connection.prepare("SELECT code,severity,description,occurrence_count,line_number FROM import_diagnostics WHERE archive_id=?1 ORDER BY id LIMIT 100").map_err(|_|error("import_database","Could not read import diagnostics."))?;
    summary.diagnostics = statement
        .query_map([id], |row| {
            Ok(Diagnostic {
                code: row.get(0)?,
                severity: row.get(1)?,
                description: row.get(2)?,
                occurrence_count: row.get(3)?,
                line_number: row.get(4)?,
            })
        })
        .map_err(|_| error("import_database", "Could not read import diagnostics."))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| error("import_database", "Could not read import diagnostics."))?;
    Ok(summary)
}
pub fn list(connection: &Connection, query: &str, offset: u32, limit: u32) -> Result<ArchivePage> {
    if query.chars().count() > 200 || limit == 0 || limit > 100 || offset > 1_000_000 {
        return Err(error(
            "invalid_filter",
            "Use a shorter archive filter and a page size from 1 to 100.",
        ));
    }
    let pattern = format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let filter = r"title LIKE ?1 ESCAPE '\' OR original_filename LIKE ?1 ESCAPE '\'";
    let total = connection
        .query_row(
            &format!("SELECT count(*) FROM archives WHERE {filter}"),
            [&pattern],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| error("import_database", "Could not count imported archives."))?;
    let mut statement=connection.prepare(&format!("SELECT {COLUMNS} FROM archives WHERE {filter} ORDER BY created_at DESC,id DESC LIMIT ?2 OFFSET ?3")).map_err(|_|error("import_database","Could not list imported archives."))?;
    let items = statement
        .query_map(params![pattern, limit, offset], row_summary)
        .map_err(|_| error("import_database", "Could not list imported archives."))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| {
            error(
                "import_database",
                "Could not read imported archive metadata.",
            )
        })?;
    Ok(ArchivePage {
        items,
        total,
        offset,
        limit,
    })
}

pub fn persist(
    connection: &Connection,
    id: &str,
    title: &str,
    session: &Session,
    parsed: &ParsedChat,
    links: &[AttachmentLink],
    bytes: u64,
) -> Result<()> {
    let fail = || {
        error(
            "import_database",
            "The import could not be saved. Database changes were rolled back.",
        )
    };
    let order = match session.order {
        DateOrder::Auto => "auto",
        DateOrder::DayFirst => "day-first",
        DateOrder::MonthFirst => "month-first",
    };
    let root = format!("archives/{id}");
    connection.execute("INSERT INTO archives(id,title,source_type,storage_path,original_filename,source_format,original_sha256,import_status,message_count,media_count,missing_media_count,total_bytes,text_encoding,date_order) VALUES (?1,?2,'chat-export',?3,?4,?5,?6,'complete',?7,?8,?9,?10,?11,?12)",params![id,title,root,session.input.filename,session.input.format,session.input.hash,parsed.messages.len() as i64,session.media.len() as i64,links.iter().filter(|link|link.media_index.is_none()).count() as i64,i64::try_from(bytes).map_err(|_|fail())?,parsed.encoding,order]).map_err(|_|fail())?;
    let conversation = uuid::Uuid::new_v4().to_string();
    let participants = serde_json::to_string(&parsed.participants).map_err(|_| fail())?;
    connection
        .execute(
            "INSERT INTO conversations(id,archive_id,title,participants_json) VALUES (?1,?2,?3,?4)",
            params![conversation, id, title, participants],
        )
        .map_err(|_| fail())?;
    let mut media_ids = Vec::new();
    for media in &session.media {
        let media_id = uuid::Uuid::new_v4().to_string();
        connection.execute("INSERT INTO archive_media(id,archive_id,original_filename,storage_path,media_type,file_size) VALUES (?1,?2,?3,?4,?5,?6)",params![media_id,id,media.original_name,format!("{root}/media/{}",media.stored_name),media.media_type,media.size as i64]).map_err(|_|fail())?;
        media_ids.push(media_id);
    }
    let mut message_statement=connection.prepare("INSERT INTO messages(id,conversation_id,sender,message_text,timestamp,original_timestamp,local_timestamp,timezone_status,classification,original_record,sequence_number) VALUES (?1,?2,?3,?4,NULL,?5,?6,'unknown',?7,?8,?9)").map_err(|_|fail())?;
    let mut attachment_statement=connection.prepare("INSERT INTO attachments(id,message_id,media_type,storage_path,file_size,original_filename,archive_media_id,is_missing) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)").map_err(|_|fail())?;
    let mut linked = links.iter().peekable();
    for (index, message) in parsed.messages.iter().enumerate() {
        let message_id = uuid::Uuid::new_v4().to_string();
        message_statement
            .execute(params![
                message_id,
                conversation,
                message.sender,
                message.text,
                message.original_timestamp,
                message.local_timestamp,
                message.classification,
                message.original_record,
                index as i64
            ])
            .map_err(|_| fail())?;
        while linked
            .peek()
            .is_some_and(|link| link.message_index == index)
        {
            let link = linked.next().unwrap();
            let media = link.media_index.map(|index| &session.media[index]);
            attachment_statement
                .execute(params![
                    uuid::Uuid::new_v4().to_string(),
                    message_id,
                    media.map(|value| value.media_type.as_str()),
                    media.map(|value| format!("{root}/media/{}", value.stored_name)),
                    media.map(|value| value.size as i64),
                    link.original_name,
                    link.media_index.map(|index| media_ids[index].as_str()),
                    i32::from(media.is_none())
                ])
                .map_err(|_| fail())?;
        }
    }
    for diagnostic in &parsed.diagnostics {
        connection.execute("INSERT INTO import_diagnostics(archive_id,code,severity,description,occurrence_count,line_number) VALUES (?1,?2,?3,?4,?5,?6)",params![id,diagnostic.code,diagnostic.severity,diagnostic.description,diagnostic.occurrence_count,diagnostic.line_number]).map_err(|_|fail())?;
    }
    Ok(())
}
