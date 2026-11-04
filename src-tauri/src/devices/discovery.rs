use super::{
    adapter::{Budget, Content, Object},
    models::{Discovery, Finding, Result, error, now},
};
use std::{cell::RefCell, collections::HashMap};

// Reuse a bounded operation-local directory observation for shared candidate prefixes.
// No WPD identifiers survive the operation and no media leaf is recursively traversed.
struct Cached<'a> {
    inner: &'a dyn Content,
    folders: RefCell<HashMap<String, Vec<Object>>>,
}
impl Content for Cached<'_> {
    fn identity(&self) -> super::adapter::Identity {
        self.inner.identity()
    }
    fn children(&self, parent: &str, budget: &mut Budget) -> Result<Vec<Object>> {
        budget.check()?;
        if let Some(objects) = self.folders.borrow().get(parent) {
            return Ok(objects.clone());
        }
        let objects = self.inner.children(parent, budget)?;
        self.folders
            .borrow_mut()
            .insert(parent.to_owned(), objects.clone());
        Ok(objects)
    }
    fn probe(&self, parent: &str, budget: &mut Budget) -> Result<()> {
        self.inner.probe(parent, budget)
    }
}

pub fn storage_roots(content: &dyn Content, budget: &mut Budget) -> Result<Vec<Object>> {
    let roots: Vec<_> = content
        .children("DEVICE", budget)?
        .into_iter()
        .filter(|o| o.storage)
        .take(9)
        .collect();
    if roots.len() > 8 {
        return Err(error("limit"));
    }
    Ok(roots)
}
fn folder(
    content: &dyn Content,
    parent: &str,
    path: &[&str],
    budget: &mut Budget,
) -> Result<Option<Object>> {
    let mut id = parent.to_owned();
    let mut found = None;
    for segment in path {
        budget.check()?;
        // Never descend an arbitrary object: only an exact candidate segment.
        let matches: Vec<_> = content
            .children(&id, budget)?
            .into_iter()
            .filter(|o| o.folder && o.name.eq_ignore_ascii_case(segment))
            .collect();
        if matches.len() > 1 {
            return Err(error("limit"));
        }
        let Some(object) = matches.into_iter().next() else {
            return Ok(None);
        };
        id = object.id.clone();
        found = Some(object);
    }
    Ok(found)
}
pub fn discover(content: &dyn Content, budget: &mut Budget) -> Discovery {
    let cached = Cached {
        inner: content,
        folders: RefCell::new(HashMap::new()),
    };
    let content: &dyn Content = &cached;
    let mut report = Discovery {
        status: "complete",
        checked_at: now(),
        examined: 0,
        findings: vec![],
        diagnostic: None,
    };
    let outcome = (|| -> Result<()> {
        let roots = storage_roots(content, budget)?;
        if roots.is_empty() {
            return Err(error("access_unavailable"));
        }
        for (root_index, root) in roots.iter().enumerate() {
            for (application, package, base) in [
                ("WhatsApp", "com.whatsapp", "WhatsApp"),
                ("WhatsApp Business", "com.whatsapp.w4b", "WhatsApp Business"),
            ] {
                for segments in [vec!["Android", "media", package, base], vec![base]] {
                    let location = format!("Storage {} / {}", root_index + 1, segments.join(" / "));
                    let candidate = match folder(content, &root.id, &segments, budget) {
                        Ok(candidate) => candidate,
                        Err(e) => {
                            report.findings.push(Finding {
                                application,
                                location,
                                category: "candidate",
                                status: e.code,
                                encrypted_backup_found: None,
                                diagnostic: Some(e.clone()),
                            });
                            if matches!(e.code, "access_denied" | "failed") {
                                report.status = "incomplete";
                                continue;
                            }
                            return Err(e);
                        }
                    };
                    let Some(candidate) = candidate else {
                        report.findings.push(Finding {
                            application,
                            location,
                            category: "candidate",
                            status: "not_found",
                            encrypted_backup_found: None,
                            diagnostic: None,
                        });
                        continue;
                    };
                    report.findings.push(Finding {
                        application,
                        location: location.clone(),
                        category: "candidate",
                        status: "found",
                        encrypted_backup_found: None,
                        diagnostic: None,
                    });
                    for (category, leaf) in [
                        ("encrypted_databases", "Databases"),
                        ("images", "Images"),
                        ("videos", "Video"),
                        ("documents", "Documents"),
                        ("audio", "Audio"),
                        ("voice_notes", "Voice Notes"),
                    ] {
                        let media_leaf = format!("{base} {leaf}");
                        let path = if category == "encrypted_databases" {
                            vec!["Databases"]
                        } else {
                            vec!["Media", media_leaf.as_str()]
                        };
                        let location = format!("{location} / {}", path.join(" / "));
                        let result =
                            folder(content, &candidate.id, &path, budget).and_then(|found| {
                                found
                                    .map(|found| {
                                        if category == "encrypted_databases" {
                                            content.children(&found.id, budget)
                                        } else {
                                            content.probe(&found.id, budget).map(|_| vec![])
                                        }
                                    })
                                    .transpose()
                            });
                        match result {
                            Ok(objects) => {
                                let encrypted = if category == "encrypted_databases" {
                                    objects.as_ref().map(|items| {
                                        items.iter().any(|o| !o.folder && encrypted_name(&o.name))
                                    })
                                } else {
                                    None
                                };
                                report.findings.push(Finding {
                                    application,
                                    location,
                                    category,
                                    status: if objects.is_some() {
                                        "accessible"
                                    } else {
                                        "not_found"
                                    },
                                    encrypted_backup_found: encrypted,
                                    diagnostic: None,
                                });
                            }
                            Err(e) => {
                                report.findings.push(Finding {
                                    application,
                                    location,
                                    category,
                                    status: e.code,
                                    encrypted_backup_found: None,
                                    diagnostic: Some(e.clone()),
                                });
                                report.status = "incomplete";
                                if !matches!(e.code, "access_denied" | "failed") {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = outcome {
        report.status = match e.code {
            "cancelled" => "cancelled",
            "disconnected" => "disconnected",
            "unsupported" => "unsupported",
            _ => "incomplete",
        };
        report.diagnostic = Some(e);
    }
    report.examined = budget.examined;
    report
}
fn encrypted_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("msgstore")
        && name.rsplit_once(".crypt").is_some_and(|(_, suffix)| {
            !suffix.is_empty() && suffix.len() <= 3 && suffix.bytes().all(|b| b.is_ascii_digit())
        })
}
