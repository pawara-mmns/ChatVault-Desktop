use super::{files::safe_component, models::*};
use crate::devices::adapter::{Budget, Content, FileMetadata, Object};
use std::collections::HashSet;
pub const MAX_FILES: usize = 50_000;
pub const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024 * 1024;
#[derive(Clone)]
pub struct PlannedFile {
    pub object_id: String,
    pub entry: FileEntry,
}
pub struct Plan {
    pub summary: PlanSummary,
    pub files: Vec<PlannedFile>,
    pub created: u64,
}
fn children(content: &dyn Content, id: &str, budget: &mut Budget) -> Result<Vec<Object>> {
    let mut list = vec![];
    let result = content.visit_children(id, budget, &mut |o| {
        if list.len() >= MAX_FILES {
            return Err(crate::devices::models::error("limit"));
        }
        list.push(o);
        Ok(())
    });
    result?;
    Ok(list)
}
fn find(
    content: &dyn Content,
    root: &str,
    path: &[&str],
    budget: &mut Budget,
) -> Result<Option<Object>> {
    let mut parent = root.to_owned();
    let mut found = None;
    for segment in path {
        let Some(object) = content.find_folder(&parent, segment, budget)? else {
            return Ok(None);
        };
        parent = object.id.clone();
        found = Some(object);
    }
    Ok(found)
}
#[cfg(test)]
pub fn inventory(
    content: &dyn Content,
    device_id: String,
    name: Option<String>,
    source: Source,
    vault: String,
    available: Option<u64>,
    budget: &mut Budget,
) -> Result<Plan> {
    inventory_with_options(
        content,
        device_id,
        name,
        source,
        vault,
        available,
        budget,
        &ScanOptions::default(),
    )
}
#[allow(clippy::too_many_arguments)]
pub fn inventory_with_options(
    content: &dyn Content,
    device_id: String,
    name: Option<String>,
    source: Source,
    vault: String,
    available: Option<u64>,
    budget: &mut Budget,
    options: &ScanOptions,
) -> Result<Plan> {
    options.validate()?;
    let mut plan = Plan {
        summary: PlanSummary {
            scanned_categories: options.categories.clone(),
            skipped_entries: 0,
            inaccessible_locations: 0,
            id: uuid::Uuid::new_v4().to_string(),
            device_id,
            device_name: name,
            source,
            complete: true,
            warnings: vec![],
            categories: vec![],
            total_files: 0,
            known_bytes: 0,
            unknown_sizes: 0,
            available_bytes: available,
            vault_path: vault,
        },
        files: vec![],
        created: crate::devices::models::now(),
    };
    let roots: Vec<_> = children(content, "DEVICE", budget)?
        .into_iter()
        .filter(|o| o.storage)
        .collect();
    if roots.is_empty() {
        return Err(fail("access_denied"));
    }
    if roots.len() > 8 {
        return Err(fail("limit"));
    }
    let mut visited = HashSet::new();
    for (index, root) in roots.iter().enumerate() {
        for (layout, path) in [
            (
                "current",
                vec!["Android", "media", source.package(), source.base()],
            ),
            ("legacy", vec![source.base()]),
        ] {
            let candidate = match find(content, &root.id, &path, budget) {
                Ok(v) => v,
                Err(e) => {
                    restriction(&mut plan, &e);
                    continue;
                }
            };
            let Some(candidate) = candidate else { continue };
            let prefix = format!("Storage {}/{layout}/{}", index + 1, source.base());
            if options.includes(Category::Databases) {
                match find(content, &candidate.id, &["Databases"], budget) {
                    Ok(Some(folder)) => walk(
                        content,
                        folder,
                        Category::Databases,
                        format!("{prefix}/Databases"),
                        "databases".into(),
                        budget,
                        &mut plan,
                        &mut visited,
                        options,
                    ),
                    Ok(None) => (),
                    Err(e) => restriction(&mut plan, &e),
                }
            }
            if options.categories == vec![Category::Databases] {
                continue;
            }
            match find(content, &candidate.id, &["Media"], budget) {
                Ok(Some(media)) => match children(content, &media.id, budget) {
                    Ok(objects) => {
                        for object in objects {
                            if !object.folder {
                                continue;
                            }
                            let category = category(&object.name, source);
                            if !options.includes(category) {
                                continue;
                            }
                            let source_path = format!("{prefix}/Media/{}", object.name);
                            let local = format!(
                                "media/{}/{}",
                                category.key(),
                                safe_component(&object.name)
                            );
                            walk(
                                content,
                                object,
                                category,
                                source_path,
                                local,
                                budget,
                                &mut plan,
                                &mut visited,
                                options,
                            );
                        }
                    }
                    Err(e) => restriction(&mut plan, &e),
                },
                Ok(None) => (),
                Err(e) => restriction(&mut plan, &e),
            }
        }
    }
    for file in &plan.files {
        let Some(summary) = plan
            .summary
            .categories
            .iter_mut()
            .find(|c| c.category == file.entry.category)
        else {
            plan.summary.categories.push(CategorySummary {
                category: file.entry.category,
                files: 1,
                known_bytes: file.entry.expected_size.unwrap_or(0),
                unknown_sizes: usize::from(file.entry.expected_size.is_none()),
            });
            continue;
        };
        summary.files += 1;
        summary.known_bytes += file.entry.expected_size.unwrap_or(0);
        summary.unknown_sizes += usize::from(file.entry.expected_size.is_none());
    }
    plan.summary.total_files = plan.files.len();
    plan.summary.known_bytes = plan
        .files
        .iter()
        .map(|f| f.entry.expected_size.unwrap_or(0))
        .sum();
    plan.summary.unknown_sizes = plan
        .files
        .iter()
        .filter(|f| f.entry.expected_size.is_none())
        .count();
    if plan.summary.known_bytes > MAX_TOTAL_BYTES {
        return Err(fail("limit"));
    }
    if let Err(e) = budget.check() {
        if e.code == "cancelled" {
            return Err(e.into());
        }
        restriction(&mut plan, &e.into());
    }
    if plan.summary.warnings.iter().any(|w| w == "disconnected") {
        return Err(fail("disconnected"));
    }
    Ok(plan)
}
fn category(name: &str, source: Source) -> Category {
    let name = name.to_lowercase();
    for (suffix, category) in [
        ("images", Category::Images),
        ("video", Category::Videos),
        ("documents", Category::Documents),
        ("audio", Category::Audio),
        ("voice notes", Category::VoiceNotes),
    ] {
        if name == format!("{} {suffix}", source.base().to_lowercase()) {
            return category;
        }
    }
    Category::Other
}
fn restriction(plan: &mut Plan, e: &Failure) {
    plan.summary.complete = false;
    plan.summary.inaccessible_locations += 1;
    if let Some(diagnostic) = &e.diagnostic {
        if !plan.summary.warnings.contains(diagnostic) {
            plan.summary.warnings.push(diagnostic.clone());
        }
    }
    let code = e.code.clone();
    if !plan.summary.warnings.contains(&code) && plan.summary.warnings.len() < 100 {
        plan.summary.warnings.push(code);
    }
}
#[allow(clippy::too_many_arguments)]
fn walk(
    content: &dyn Content,
    root: Object,
    category: Category,
    source: String,
    local: String,
    budget: &mut Budget,
    plan: &mut Plan,
    visited: &mut HashSet<String>,
    options: &ScanOptions,
) {
    let mut pending = vec![(root, source, local, 0)];
    while let Some((folder, source, local, depth)) = pending.pop() {
        if depth > options.max_depth || plan.files.len() >= options.max_files {
            restriction(plan, &fail("limit"));
            break;
        }
        if !visited.insert(folder.id.clone()) {
            restriction(plan, &fail("duplicate_object"));
            continue;
        }
        // Preserve already returned entries when a later Next/GetValues fails.
        let mut objects = vec![];
        let trace = budget.trace.clone();
        let result = content.visit_children(&folder.id, budget, &mut |mut o| {
            if plan.files.len() >= options.max_files {
                return Err(crate::devices::models::error("limit"));
            }
            if !o.folder {
                if let Some(metadata) = o.metadata.take() {
                    record_file(
                        plan,
                        visited,
                        o.id,
                        metadata,
                        category,
                        &source,
                        &local,
                        trace.as_ref(),
                    );
                    return Ok(());
                }
            }
            if objects.len() >= options.max_objects {
                return Err(crate::devices::models::error("limit"));
            }
            objects.push(o);
            Ok(())
        });
        if let Err(e) = result {
            restriction(plan, &e.into());
        }
        for object in objects {
            if plan.files.len() >= options.max_files {
                restriction(plan, &fail("limit"));
                break;
            }
            if object.folder {
                let next_source = format!("{source}/{}", object.name);
                let next_local = format!(
                    "{local}/{}-{}",
                    safe_component(&object.name),
                    &uuid::Uuid::new_v4().to_string()[..8]
                );
                pending.push((object, next_source, next_local, depth + 1));
                continue;
            }
            let metadata_result = match object.metadata {
                Some(metadata) => Ok(metadata),
                None => budget
                    .check()
                    .and_then(|_| content.file_metadata(&object.id, budget)),
            };
            let metadata = match metadata_result {
                Ok(v) => v,
                Err(e) => {
                    restriction(plan, &e.into());
                    continue;
                }
            };
            record_file(
                plan,
                visited,
                object.id,
                metadata,
                category,
                &source,
                &local,
                budget.trace.as_ref(),
            );
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn record_file(
    plan: &mut Plan,
    visited: &mut HashSet<String>,
    object_id: String,
    metadata: FileMetadata,
    category: Category,
    source: &str,
    local: &str,
    trace: Option<&crate::devices::scan::ScanTrace>,
) {
    if !visited.insert(object_id.clone()) {
        restriction(plan, &fail("duplicate_object"));
        return;
    }
    if category == Category::Databases && !encrypted(&metadata.name) {
        plan.summary.skipped_entries += 1;
        return;
    }
    if metadata.size.is_some_and(|s| s > MAX_FILE_BYTES)
        || source.len() + metadata.name.len() > 2048
    {
        restriction(plan, &fail("limit"));
        return;
    }
    if let Some(trace) = trace {
        trace.inventory(metadata.size);
    }
    let id = uuid::Uuid::new_v4().to_string();
    plan.files.push(PlannedFile {
        object_id,
        entry: FileEntry {
            id: id.clone(),
            category,
            source_relative: format!("{source}/{}", metadata.name),
            local_relative: format!("{local}/{id}-{}", safe_component(&metadata.name)),
            original_name: metadata.name,
            expected_size: metadata.size,
            actual_bytes: 0,
            sha256: None,
            local_verification: "pending".into(),
            source_verification: "not_requested".into(),
            size_validation: "unknown".into(),
            state: "pending".into(),
            error_code: None,
        },
    });
}

fn encrypted(name: &str) -> bool {
    name.to_ascii_lowercase()
        .rsplit_once(".crypt")
        .is_some_and(|(_, s)| {
            !s.is_empty() && s.len() <= 3 && s.bytes().all(|b| b.is_ascii_digit())
        })
}
pub fn select(plan: &Plan, selection: &Selection) -> Result<Vec<PlannedFile>> {
    if selection.categories.is_empty() || selection.categories.len() > 7 {
        return Err(fail("invalid_selection"));
    }
    if !plan.summary.complete && !selection.acknowledge_incomplete {
        return Err(fail("incomplete_inventory"));
    }
    let selected: HashSet<_> = selection
        .file_ids
        .as_ref()
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();
    if selected.len() > 10_000
        || selection
            .file_ids
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.len() != selected.len())
    {
        return Err(fail("invalid_selection"));
    }
    let files: Vec<_> = plan
        .files
        .iter()
        .filter(|f| {
            selection.categories.contains(&f.entry.category)
                && (selection.file_ids.is_none() || selected.contains(&f.entry.id))
        })
        .cloned()
        .collect();
    if files.is_empty() || selection.file_ids.is_some() && files.len() != selected.len() {
        return Err(fail("invalid_selection"));
    }
    Ok(files)
}
