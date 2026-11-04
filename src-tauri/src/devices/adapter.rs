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
}
impl Budget {
    pub fn new(cancel: Arc<AtomicBool>) -> Self {
        Self {
            cancel,
            deadline: Instant::now() + Duration::from_secs(25),
            examined: 0,
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Acquire) {
            Err(error("cancelled"))
        } else if Instant::now() >= self.deadline {
            Err(error("timeout"))
        } else if self.examined >= MAX_ENTRIES {
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
