use super::{
    files,
    models::*,
    planner::{MAX_FILE_BYTES, MAX_TOTAL_BYTES, PlannedFile},
};
use crate::devices::adapter::{Budget, Content, DeviceRead};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};
pub fn copy(
    content: &dyn Content,
    file: &PlannedFile,
    root: &Path,
    budget: &mut Budget,
    reread: bool,
    p: &mut Progress,
    update: &mut dyn FnMut(&Progress),
) -> Result<FileEntry> {
    let mut entry = file.entry.clone();
    let before = content.file_metadata(&file.object_id, budget)?;
    if before.name != entry.original_name || before.size != entry.expected_size {
        return Err(fail("source_changed"));
    }
    let target = files::join(root, &entry.local_relative)?;
    crate::storage::paths::create_safe_directory(
        target.parent().ok_or_else(|| fail("unsafe_path"))?,
    )?;
    let temp = target.with_file_name(format!(".partial-{}", entry.id));
    let mut output = files::create_file(&temp)?;
    let mut stream = content.read_stream(&file.object_id, budget)?;
    let mut buffer = vec![0; stream.optimal_size().clamp(16 * 1024, 512 * 1024)];
    p.phase = "copying".into();
    p.current_filename = Some(entry.original_name.clone());
    update(p);
    let (bytes, incoming) = receive(stream.as_mut(), &mut output, &mut buffer, budget, p, update)?;
    drop(stream);
    output.flush().and_then(|_| output.sync_all()).map_err(io)?;
    drop(output);
    if entry.expected_size.is_some_and(|size| size != bytes) {
        return Err(fail("source_changed"));
    }
    p.phase = "verifying".into();
    update(p);
    let verified_before = p.verified_bytes;
    let (local_bytes, local_hash) = files::hash_local(&temp, budget, |n| {
        p.verified_bytes = verified_before + n;
        update(p);
    })?;
    if local_bytes != bytes || local_hash != incoming {
        return Err(fail("hash_mismatch"));
    }
    let after = content.file_metadata(&file.object_id, budget)?;
    if before != after {
        return Err(fail("source_changed"));
    }
    entry.source_verification = if reread {
        let mut source = content.read_stream(&file.object_id, budget)?;
        let mut hash = Sha256::new();
        let mut count = 0;
        loop {
            budget.check()?;
            let n = source.read(&mut buffer)?;
            if n > buffer.len() {
                return Err(fail("failed"));
            }
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > MAX_FILE_BYTES {
                return Err(fail("limit"));
            }
            hash.update(&buffer[..n]);
            update(p);
        }
        if count != bytes
            || format!("{:x}", hash.finalize()) != incoming
            || content.file_metadata(&file.object_id, budget)? != before
        {
            return Err(fail("source_changed"));
        }
        "verified".into()
    } else {
        "not_requested".into()
    };
    budget.check()?;
    files::rename_new(&temp, &target)?;
    entry.actual_bytes = bytes;
    entry.sha256 = Some(incoming);
    entry.local_verification = "verified".into();
    entry.size_validation = if entry.expected_size.is_some() {
        "matched"
    } else {
        "unknown"
    }
    .into();
    entry.state = "verified".into();
    Ok(entry)
}

// A fallible writer boundary lets tests exercise disk-full, removal and write
// failures without filling or changing permissions on the developer's disk.
pub(super) fn receive(
    stream: &mut dyn DeviceRead,
    output: &mut dyn Write,
    buffer: &mut [u8],
    budget: &Budget,
    p: &mut Progress,
    update: &mut dyn FnMut(&Progress),
) -> Result<(u64, String)> {
    let mut hash = Sha256::new();
    let mut bytes = 0;
    loop {
        budget.check()?;
        let n = stream.read(buffer)?;
        if n > buffer.len() {
            return Err(fail("failed"));
        }
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > MAX_FILE_BYTES || p.copied_bytes.saturating_add(n as u64) > MAX_TOTAL_BYTES {
            return Err(fail("limit"));
        }
        output.write_all(&buffer[..n]).map_err(io)?;
        p.copied_bytes += n as u64;
        hash.update(&buffer[..n]);
        update(p);
    }
    Ok((bytes, format!("{:x}", hash.finalize())))
}
