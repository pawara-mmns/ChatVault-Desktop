use super::{
    Result, archive_repository, error,
    input::{self, Input},
    media,
    models::*,
    parser,
    staging::{self, Stage},
    zip_processor::{self, MediaFile},
};
use crate::storage::{paths, service::StorageService};
use rusqlite::{TransactionBehavior, params};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Session {
    pub stage: Stage,
    pub input: Input,
    pub selection: SelectedImport,
    pub vault: PathBuf,
    pub text_path: Option<PathBuf>,
    pub media: Vec<MediaFile>,
    pub extraction_diagnostics: Vec<Diagnostic>,
    pub parsed: Option<ParsedChat>,
    pub order: DateOrder,
    created: Instant,
    _coordinator: fs::File,
}

impl StorageService {
    pub(crate) fn ensure_import_selectable(&mut self) -> Result<()> {
        if self
            .import_session
            .as_ref()
            .is_some_and(|session| session.created.elapsed() > Duration::from_secs(30 * 60))
        {
            self.import_session.take();
        }
        if self.import_session.is_some() {
            return Err(error(
                "import_busy",
                "Cancel the current preview before choosing another export.",
            ));
        }
        let _coordinator = self.import_coordinator()?;
        self.import_vault()?;
        Ok(())
    }
    fn valid_token(&mut self, token: &str) -> Result<()> {
        if self
            .import_session
            .as_ref()
            .is_some_and(|session| session.created.elapsed() > Duration::from_secs(30 * 60))
        {
            self.import_session.take();
        }
        if !self
            .import_session
            .as_ref()
            .is_some_and(|session| session.selection.token == token)
        {
            return Err(error(
                "import_expired",
                "This import session is no longer available. Choose the export again.",
            ));
        }
        Ok(())
    }
    pub(super) fn select_source(&mut self, source: &Path) -> Result<SelectedImport> {
        if self
            .import_session
            .as_ref()
            .is_some_and(|session| session.created.elapsed() > Duration::from_secs(30 * 60))
        {
            self.import_session.take();
        }
        if self.import_session.is_some() {
            return Err(error(
                "import_busy",
                "Cancel the current preview before choosing another export.",
            ));
        }
        let coordinator = self.import_coordinator()?;
        self.recover_imports_locked()?;
        let (vault, installation) = self.import_vault()?;
        let stage = Stage::new(&vault, &installation)?;
        let input = input::copy_source(source, &stage, &self.import_limits)?;
        let duplicate = archive_repository::duplicate(&self.connection, &input.hash)?;
        let selection = SelectedImport {
            token: stage.id.clone(),
            filename: input.filename.clone(),
            size_bytes: input.size,
            source_format: input.format.clone(),
            duplicate_archive: duplicate,
        };
        self.import_session = Some(Session {
            stage,
            input,
            selection: selection.clone(),
            vault,
            text_path: None,
            media: Vec::new(),
            extraction_diagnostics: Vec::new(),
            parsed: None,
            order: DateOrder::Auto,
            created: Instant::now(),
            _coordinator: coordinator,
        });
        Ok(selection)
    }
    pub fn analyze_import(&mut self, token: &str, order: DateOrder) -> Result<ImportPreview> {
        self.valid_token(token)?;
        let mut session = self.import_session.take().unwrap();
        if session.selection.duplicate_archive.is_some() {
            return Err(error(
                "duplicate_import",
                "This identical file is already imported. Cancel and open its existing archive.",
            ));
        }
        if self.import_vault()?.0 != session.vault {
            return Err(error(
                "vault_changed",
                "The vault changed during this import. Choose the export again.",
            ));
        }
        if session.text_path.is_none() {
            if session.input.format == "txt" {
                session.text_path = Some(session.input.stored.clone());
            } else {
                let extracted = zip_processor::extract(
                    &session.input.stored,
                    &session.stage,
                    &self.import_limits,
                )?;
                session.media = extracted.media;
                session.extraction_diagnostics = extracted.diagnostics;
                let mut candidates = Vec::new();
                for (_, path) in &extracted.texts {
                    let bytes = input::read_text(&path, self.import_limits.text_bytes)?;
                    match parser::parse(&bytes, order, self.import_limits.messages) {
                        Ok(_) => candidates.push(path.clone()),
                        Err(failure) if extracted.texts.len() == 1 => return Err(failure),
                        Err(_) => {}
                    }
                }
                if candidates.len() != 1 {
                    return Err(error(
                        "unsupported_format",
                        "The ZIP must contain exactly one compatible conversation text export. No conversation was silently selected.",
                    ));
                }
                session.text_path = candidates.pop();
                // Non-conversation .txt files are opaque text attachments, not discarded exports.
                for (name, path) in extracted.texts {
                    if session.text_path.as_ref() == Some(&path) {
                        continue;
                    }
                    use std::io::Write;
                    let stored_name = format!("{}.txt", uuid::Uuid::new_v4());
                    let mut output = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(session.stage.payload().join("media").join(&stored_name))
                        .map_err(|_| {
                            error("import_io", "Could not retain an exported text attachment.")
                        })?;
                    let mut file = paths::open_regular_file(&path)?;
                    let size = std::io::copy(&mut file, &mut output).map_err(|_| {
                        error("import_io", "Could not retain an exported text attachment.")
                    })?;
                    output
                        .flush()
                        .and_then(|_| output.sync_all())
                        .map_err(|_| {
                            error("import_io", "Could not sync an exported text attachment.")
                        })?;
                    session.media.push(MediaFile {
                        original_name: name,
                        stored_name,
                        size,
                        media_type: "text/plain".into(),
                    });
                }
            }
        }
        let bytes = input::read_text(
            session.text_path.as_ref().unwrap(),
            self.import_limits.text_bytes,
        )?;
        let mut parsed = parser::parse(&bytes, order, self.import_limits.messages)?;
        parsed
            .diagnostics
            .extend(session.extraction_diagnostics.clone());
        let links = media::match_references(&mut parsed, &session.media);
        let title = Path::new(&session.input.filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Imported chat")
            .trim_start_matches("WhatsApp Chat with ")
            .chars()
            .take(200)
            .collect();
        let preview = ImportPreview {
            selection: session.selection.clone(),
            title,
            message_count: parsed.messages.len(),
            participant_count: parsed.participants.len(),
            participants: parsed.participants.iter().take(20).cloned().collect(),
            media_count: session.media.len(),
            attachment_count: links.len(),
            missing_attachment_count: links
                .iter()
                .filter(|link| link.media_index.is_none())
                .count(),
            date_order_required: parsed.date_order_required,
            date_order: order,
            encoding: parsed.encoding.clone(),
            samples: parsed
                .messages
                .iter()
                .take(5)
                .map(|message| MessageSample {
                    sender: message.sender.clone(),
                    text: message.text.chars().take(2000).collect(),
                    original_timestamp: message.original_timestamp.clone(),
                    local_timestamp: message.local_timestamp.clone(),
                    classification: message.classification.clone(),
                })
                .collect(),
            diagnostics: parsed.diagnostics.clone(),
        };
        session.parsed = Some(parsed);
        session.order = order;
        self.import_session = Some(session);
        Ok(preview)
    }
    pub fn cancel_import(&mut self, token: &str) -> Result<()> {
        if self.import_session.is_none() {
            return Ok(());
        }
        if !self
            .import_session
            .as_ref()
            .is_some_and(|session| session.selection.token == token)
        {
            return Err(error(
                "import_expired",
                "This import session is no longer available.",
            ));
        }
        self.import_session.take();
        Ok(())
    }
    pub fn confirm_import(&mut self, token: &str, title: &str) -> Result<ArchiveSummary> {
        self.valid_token(token)?;
        let session = self.import_session.take().unwrap();
        let parsed = session.parsed.as_ref().ok_or_else(|| {
            error(
                "import_not_analyzed",
                "Analyze the selected export before confirming.",
            )
        })?;
        if parsed.date_order_required {
            return Err(error(
                "ambiguous_dates",
                "Select a date format and analyze again before importing.",
            ));
        }
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 200 || title.chars().any(char::is_control) {
            return Err(error(
                "invalid_title",
                "Use a conversation title from 1 to 200 characters without control characters.",
            ));
        }
        let (vault, installation) = self.import_vault()?;
        if vault != session.vault {
            return Err(error(
                "vault_changed",
                "The vault changed during this import. Nothing was saved.",
            ));
        }
        if archive_repository::duplicate(&self.connection, &session.input.hash)?.is_some() {
            return Err(error(
                "duplicate_import",
                "This exact file is already imported. No duplicate archive was created.",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let relative = format!("archives/{id}");
        let destination = vault.join(&relative);
        // Durable intent exists before any final directory, so a crash can be recovered safely.
        self.connection.execute("INSERT INTO import_jobs(id,vault_path,storage_path,installation_id) VALUES (?1,?2,?3,?4)",params![id,vault.to_str(),relative,installation]).map_err(|_|error("import_database","Could not journal the import. Nothing was saved."))?;
        if fs::create_dir(&destination).is_err() {
            let _ = self
                .connection
                .execute("DELETE FROM import_jobs WHERE id=?1", [&id]);
            return Err(error(
                "import_io",
                "Could not reserve new archive storage. Existing files were preserved.",
            ));
        }
        if let Err(failure) =
            staging::write_owner(&destination, staging::FINAL_MARKER, &id, &installation)
        {
            return Err(failure);
        }
        let saved = (|| -> Result<()> {
            staging::copy_new_tree(&session.stage.payload(), &destination)?;
            let bytes = crate::storage::usage::calculate(&destination)?.bytes;
            let transaction = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|_| error("import_database", "Could not begin the import transaction."))?;
            if archive_repository::duplicate(&transaction, &session.input.hash)?.is_some() {
                return Err(error(
                    "duplicate_import",
                    "This exact file was imported by another session. No duplicate was created.",
                ));
            }
            let configured = crate::storage::repository::setting(&transaction, "vault_path")?;
            if configured.as_deref() != vault.to_str() {
                return Err(error(
                    "vault_changed",
                    "The vault changed before the import could commit.",
                ));
            }
            // Rebuild matching diagnostics only once: parsed preview already contains them.
            let mut links_chat = parser::parse(
                &input::read_text(
                    session.text_path.as_ref().unwrap(),
                    self.import_limits.text_bytes,
                )?,
                session.order,
                self.import_limits.messages,
            )?;
            let links = media::match_references(&mut links_chat, &session.media);
            archive_repository::persist(&transaction, &id, title, &session, parsed, &links, bytes)?;
            transaction
                .execute("DELETE FROM import_jobs WHERE id=?1", [&id])
                .map_err(|_| error("import_database", "Could not finish the import journal."))?;
            transaction.commit().map_err(|_| {
                error(
                    "import_database",
                    "The import transaction could not commit.",
                )
            })?;
            Ok(())
        })();
        if let Err(failure) = saved {
            if staging::cleanup_owned(&destination, staging::FINAL_MARKER, &id, &installation)
                .is_ok()
            {
                let _ = self
                    .connection
                    .execute("DELETE FROM import_jobs WHERE id=?1", [&id]);
            }
            return Err(failure);
        }
        archive_repository::summary(&self.connection, &id)
    }
    pub fn recover_imports(&mut self) -> Result<()> {
        if self.import_session.is_some() {
            return Ok(());
        }
        let _coordinator = match self.import_coordinator() {
            Ok(value) => value,
            Err(failure) if failure.code == "import_busy" => return Ok(()),
            Err(failure) => return Err(failure),
        };
        self.recover_imports_locked()
    }
    fn recover_imports_locked(&mut self) -> Result<()> {
        let (vault, installation) = self.import_vault()?;
        staging::recover_stages(&vault, &installation)?;
        let jobs: Vec<(String, String, String, String)> = {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT id,vault_path,storage_path,installation_id FROM import_jobs LIMIT 1001",
                )
                .map_err(|_| {
                    error(
                        "import_database",
                        "Could not read the import recovery journal.",
                    )
                })?;
            statement
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map_err(|_| error("import_database", "Could not read import recovery records."))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| error("import_database", "Could not read import recovery records."))?
        };
        if jobs.len() > 1000 {
            return Err(error(
                "import_recovery",
                "Too many unfinished imports need inspection.",
            ));
        }
        for (id, location, relative, owner) in jobs {
            if uuid::Uuid::parse_str(&id).is_err()
                || relative != format!("archives/{id}")
                || Some(location.as_str()) != vault.to_str()
                || owner != installation
            {
                return Err(error(
                    "import_recovery",
                    "An unfinished import has inconsistent ownership. Files were preserved.",
                ));
            }
            let completed: bool = self
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM archives WHERE id=?1)",
                    [&id],
                    |row| row.get(0),
                )
                .map_err(|_| error("import_database", "Could not verify import recovery."))?;
            if completed {
                return Err(error(
                    "import_recovery",
                    "A recovery record references an existing archive. Existing data was preserved.",
                ));
            }
            staging::cleanup_owned(
                &vault.join(relative),
                staging::FINAL_MARKER,
                &id,
                &installation,
            )?;
            self.connection
                .execute("DELETE FROM import_jobs WHERE id=?1", [&id])
                .map_err(|_| error("import_database", "Could not finish import recovery."))?;
        }
        Ok(())
    }
    pub fn list_archives(&self, query: &str, offset: u32, limit: u32) -> Result<ArchivePage> {
        archive_repository::list(&self.connection, query, offset, limit)
    }
    pub fn archive_summary(&self, id: &str) -> Result<ArchiveSummary> {
        archive_repository::summary(&self.connection, id)
    }

    pub(crate) fn import_coordinator(&self) -> Result<fs::File> {
        use std::fs::OpenOptions;
        let parent = self
            .database_path
            .parent()
            .ok_or_else(|| error("import_io", "Could not locate the import coordinator."))?;
        paths::verify_ancestors(parent, false)?;
        let path = parent.join("import-coordinator.lock");
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(failure) if failure.kind() == std::io::ErrorKind::AlreadyExists => {
                paths::open_regular_file(&path)?;
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .map_err(|_| error("import_io", "Could not open the import coordinator."))?
            }
            Err(_) => {
                return Err(error(
                    "import_io",
                    "Could not create the import coordinator.",
                ));
            }
        };
        file.try_lock().map_err(|_| {
            error(
                "import_busy",
                "Another import or vault change is active. Finish or cancel it before continuing.",
            )
        })?;
        Ok(file)
    }
}
