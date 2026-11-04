use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceError {
    pub code: &'static str,
    pub message: &'static str,
    pub diagnostic: Option<String>,
}
pub type Result<T> = std::result::Result<T, DeviceError>;
pub fn error(code: &'static str) -> DeviceError {
    DeviceError {
        code,
        message: match code {
            "access_denied" => {
                "Windows or the phone denied access. Unlock the phone, choose File Transfer, approve its prompt, then refresh."
            }
            "disconnected" => "The device is no longer available. Reconnect it and refresh.",
            "busy" => "A device operation is still running. Cancel it or wait before trying again.",
            "cancelled" => "The scan was cancelled. No phone files were read or changed.",
            "timeout" => {
                "The scan reached its time limit. A Windows driver call may still be finishing; retry when it returns."
            }
            "limit" => {
                "The bounded metadata scan is incomplete. Narrower vendor access may be needed; this does not mean WhatsApp data is absent."
            }
            "unsupported" => {
                "This adapter requires Windows and a confirmed local USB portable device. Network and unverified transports are not opened."
            }
            "invalid_handle" => {
                "This device selection has expired. Refresh and select the device again."
            }
            "storage" => {
                "The approved device note could not be saved or loaded. Existing local data was not reset."
            }
            _ => {
                "Windows could not complete this device operation. Unlock the phone, choose File Transfer, and refresh."
            }
        },
        diagnostic: None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Unchecked,
    Available,
    Unavailable,
    PermissionRequired,
    Disconnected,
    Unsupported,
    Failed,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub category: Option<String>,
    pub android_identity: &'static str,
    pub transport: &'static str,
    pub access: Access,
    pub detected_at: u64,
    pub checked_at: Option<u64>,
    pub diagnostic: Option<DeviceError>,
    pub discovery: Option<Discovery>,
    pub saved_note_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceList {
    pub devices: Vec<Device>,
    pub scanned_at: Option<u64>,
    pub stale: bool,
    pub busy: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub application: &'static str,
    pub location: String,
    pub category: &'static str,
    pub status: &'static str,
    pub encrypted_backup_found: Option<bool>,
    pub diagnostic: Option<DeviceError>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub status: &'static str,
    pub checked_at: u64,
    pub examined: usize,
    pub findings: Vec<Finding>,
    pub diagnostic: Option<DeviceError>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownDevice {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub last_connected_at: u64,
    pub preferred: bool,
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn clean(value: &str) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(160)
        .collect();
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
