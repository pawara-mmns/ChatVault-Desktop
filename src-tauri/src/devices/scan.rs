//! Sanitized diagnostics: operation labels are code constants, never device data.
use super::models::DeviceError;
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Default, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub elapsed_ms: u64,
    pub objects: usize,
    pub directories: usize,
    pub folders: usize,
    pub files: usize,
    pub inventoried_files: usize,
    pub known_bytes: u64,
    pub operation: Option<&'static str>,
    pub operation_pending: bool,
    pub operation_elapsed_ms: u64,
    pub last_successful_operation: Option<&'static str>,
    pub successful_calls: usize,
    pub errors: usize,
    pub error_category: Option<&'static str>,
    pub diagnostic: Option<String>,
    pub slow: bool,
    pub deadline_reached: bool,
    pub timeout_seconds: u64,
    pub max_objects: usize,
    pub max_depth: usize,
}
struct Inner {
    started: Instant,
    call_started: Instant,
    report: ScanReport,
}
#[derive(Clone)]
pub struct ScanTrace(Arc<Mutex<Inner>>);
impl ScanTrace {
    pub fn new(seconds: u64, objects: usize, depth: usize) -> Self {
        Self(Arc::new(Mutex::new(Inner {
            started: Instant::now(),
            call_started: Instant::now(),
            report: ScanReport {
                timeout_seconds: seconds,
                max_objects: objects,
                max_depth: depth,
                ..Default::default()
            },
        })))
    }
    pub fn snapshot(&self) -> ScanReport {
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let mut r = inner.report.clone();
        r.elapsed_ms = inner.started.elapsed().as_millis() as u64;
        r.operation_elapsed_ms = if r.operation_pending {
            inner.call_started.elapsed().as_millis() as u64
        } else {
            0
        };
        r.slow = r.elapsed_ms >= 10_000;
        r.deadline_reached = r.elapsed_ms >= r.timeout_seconds * 1000;
        r
    }
    pub fn begin(&self, operation: &'static str) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(operation, "enum_objects" | "enumerate_children") {
            inner.report.directories += 1;
        }
        inner.call_started = Instant::now();
        inner.report.operation = Some(operation);
        inner.report.operation_pending = true;
    }
    pub fn end(&self, result: Option<&DeviceError>) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let r = &mut inner.report;
        r.operation_pending = false;
        if let Some(e) = result {
            r.errors += 1;
            r.error_category = Some(e.code);
            r.diagnostic = e.diagnostic.clone();
        } else {
            r.successful_calls += 1;
            r.last_successful_operation = r.operation;
        }
    }
    pub fn object(&self, folder: bool) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.report.objects += 1;
        if folder {
            inner.report.folders += 1;
        } else {
            inner.report.files += 1;
        }
    }
    pub fn inventory(&self, bytes: Option<u64>) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.report.inventoried_files += 1;
        inner.report.known_bytes += bytes.unwrap_or(0);
    }
}
