use super::models::{Result, error};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub const MAX_DEVICES: usize = 32;
pub const MAX_ENTRIES: usize = 2500;
pub const MAX_CHILDREN: usize = 256;
pub struct Budget {
    pub cancel: Arc<AtomicBool>,
    deadline: Instant,
    pub examined: usize,
    max_entries: usize,
    pub trace: Option<super::scan::ScanTrace>,
}
impl Budget {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self {
            cancel,
            deadline: Instant::now() + Duration::from_secs(25),
            examined: 0,
            max_entries: MAX_ENTRIES,
            trace: None,
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Acquire) {
            Err(error("cancelled"))
        } else if Instant::now() >= self.deadline {
            Err(error("timeout"))
        } else if self.examined >= self.max_entries {
            Err(error("limit"))
        } else {
            Ok(())
        }
    }
    pub fn entry(&mut self) -> Result<()> {
        self.check()?;
        self.examined += 1;
        Ok(())
    }
    pub(crate) fn backup(cancel: Arc<AtomicBool>, seconds: u64) -> Self {
        Self {
            cancel,
            deadline: Instant::now() + Duration::from_secs(seconds),
            examined: 0,
            max_entries: 100_001,
            trace: None,
        }
    }
    pub(crate) fn scan(
        cancel: Arc<AtomicBool>,
        seconds: u64,
        objects: usize,
        trace: super::scan::ScanTrace,
    ) -> Self {
        let mut budget = Self::backup(cancel, seconds);
        budget.max_entries = objects;
        budget.trace = Some(trace);
        budget
    }
    pub fn operation<T>(&self, name: &'static str, call: impl FnOnce() -> Result<T>) -> Result<T> {
        self.check()?;
        if let Some(trace) = &self.trace {
            trace.begin(name);
        }
        let result = call();
        if let Some(trace) = &self.trace {
            trace.end(result.as_ref().err());
        }
        result
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub name: String,
    pub size: Option<u64>,
}
pub trait DeviceRead {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize>;
    fn optimal_size(&self) -> usize {
        128 * 1024
    }
}
// Opaque Windows/object identifiers never cross IPC or enter persistence/logs.
pub struct RawDevice {
    pub key: String,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub usb: bool,
}
#[derive(Clone)]
pub struct Object {
    pub id: String,
    pub name: String,
    pub folder: bool,
    pub storage: bool,
    pub metadata: Option<FileMetadata>,
}
#[derive(Default)]
pub struct Identity {
    pub category: Option<String>,
    pub protocol: Option<String>,
}
pub trait Content {
    fn identity(&self) -> Identity;
    fn children(&self, parent: &str, budget: &mut Budget) -> Result<Vec<Object>>;
    fn probe(&self, parent: &str, budget: &mut Budget) -> Result<()> {
        self.children(parent, budget).map(|_| ())
    }
    fn visit_children(
        &self,
        parent: &str,
        budget: &mut Budget,
        visitor: &mut dyn FnMut(Object) -> Result<()>,
    ) -> Result<()> {
        let trace = budget.trace.clone();
        if let Some(trace) = &trace {
            trace.begin("enumerate_children");
        }
        let children = self.children(parent, budget);
        if let Some(trace) = &trace {
            trace.end(children.as_ref().err());
        }
        for object in children? {
            budget.check()?;
            if let Some(trace) = &trace {
                trace.object(object.folder || object.storage);
            }
            visitor(object)?;
        }
        Ok(())
    }
    fn file_metadata(&self, _: &str, _: &mut Budget) -> Result<FileMetadata> {
        Err(error("unsupported"))
    }
    fn find_folder(&self, parent: &str, name: &str, budget: &mut Budget) -> Result<Option<Object>> {
        let mut found = None;
        let result = self.visit_children(parent, budget, &mut |o| {
            if o.folder && o.name.eq_ignore_ascii_case(name) {
                found = Some(o);
                return Err(error("enumeration_stop"));
            }
            Ok(())
        });
        match result {
            Err(e) if e.code == "enumeration_stop" => Ok(found),
            Err(e) => Err(e),
            Ok(()) => Ok(found),
        }
    }
    fn read_stream(&self, _: &str, _: &mut Budget) -> Result<Box<dyn DeviceRead>> {
        Err(error("unsupported"))
    }
}
pub trait DeviceAdapter: Send + Sync {
    fn enumerate(&self, budget: &mut Budget) -> Result<Vec<RawDevice>>;
    fn open(&self, key: &str, budget: &mut Budget) -> Result<Box<dyn Content>>;
}
pub struct WindowsPortableDeviceAdapter;
#[cfg(not(windows))]
impl DeviceAdapter for WindowsPortableDeviceAdapter {
    fn enumerate(&self, _: &mut Budget) -> Result<Vec<RawDevice>> {
        Err(error("unsupported"))
    }
    fn open(&self, _: &str, _: &mut Budget) -> Result<Box<dyn Content>> {
        Err(error("unsupported"))
    }
}
