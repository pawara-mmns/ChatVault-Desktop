//! All COM interfaces are created, used, released and uninitialized on one worker.
use super::{
    adapter::*,
    models::{Result, clean, error},
};
use windows::{
    Win32::{
        Devices::{DeviceAndDriverInstallation::*, PortableDevices::*, Properties::*},
        Foundation::PROPERTYKEY,
        System::Com::*,
    },
    core::{PCWSTR, PWSTR, w},
};

struct Apartment;
impl Apartment {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(native)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct Allocated(PWSTR);
impl Allocated {
    fn text(&self) -> Result<String> {
        if self.0.is_null() {
            return Err(error("failed"));
        }
        // WPD supplies NUL-terminated CoTaskMem strings. Bound copying into Rust memory.
        let mut units = Vec::new();
        for i in 0..4096 {
            let c = unsafe { *self.0.0.add(i) };
            if c == 0 {
                return String::from_utf16(&units).map_err(|_| error("failed"));
            }
            units.push(c);
        }
        Err(error("limit"))
    }
}
impl Drop for Allocated {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0.0.cast()));
        }
    }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn native(e: windows::core::Error) -> super::models::DeviceError {
    let code = e.code().0 as u32;
    let mut result = error(match code {
        0x80070005 => "access_denied",
        0x8007048f | 0x80070490 | 0x80070015 | 0x8007001f | 0x802a0002 => "disconnected",
        0x80004001 | 0x80070032 | 0x80040154 => "unsupported",
        _ => "failed",
    });
    // Numeric HRESULT only. Never include Windows messages containing hardware IDs.
    result.diagnostic = Some(format!("WPD HRESULT 0x{code:08X}"));
    result
}
fn keys(properties: &[PROPERTYKEY]) -> Result<IPortableDeviceKeyCollection> {
    unsafe {
        let keys: IPortableDeviceKeyCollection =
            CoCreateInstance(&PortableDeviceKeyCollection, None, CLSCTX_INPROC_SERVER)
                .map_err(native)?;
        for key in properties {
            keys.Add(key).map_err(native)?;
        }
        Ok(keys)
    }
}
fn string(values: &IPortableDeviceValues, key: &PROPERTYKEY) -> Option<String> {
    unsafe {
        values
            .GetStringValue(key)
            .ok()
            .and_then(|p| Allocated(p).text().ok())
            .and_then(|s| clean(&s))
    }
}
fn original_string(values: &IPortableDeviceValues, key: &PROPERTYKEY) -> Option<String> {
    unsafe {
        values
            .GetStringValue(key)
            .ok()
            .and_then(|p| Allocated(p).text().ok())
            .filter(|s| !s.is_empty() && s.len() <= 1024)
    }
}
// Check the local PnP ancestry before opening WPD. No network/Bluetooth device is opened.
fn usb_ancestor(id: &str) -> bool {
    let wide = wide(id);
    let mut node = 0;
    unsafe {
        if CM_Locate_DevNodeW(&mut node, PCWSTR(wide.as_ptr()), CM_LOCATE_DEVNODE_NORMAL)
            != CR_SUCCESS
        {
            // GetDevices may return a device-interface symbolic link rather than a
            // devnode instance ID. Resolve it through CfgMgr32, never parse an ID.
            let mut instance = [0u16; 512];
            let mut bytes = std::mem::size_of_val(&instance) as u32;
            let mut property_type = DEVPROPTYPE(0);
            if CM_Get_Device_Interface_PropertyW(
                PCWSTR(wide.as_ptr()),
                &DEVPKEY_Device_InstanceId,
                &mut property_type,
                Some(instance.as_mut_ptr().cast()),
                &mut bytes,
                0,
            ) != CR_SUCCESS
                || property_type != DEVPROP_TYPE_STRING
                || !instance.contains(&0)
                || CM_Locate_DevNodeW(
                    &mut node,
                    PCWSTR(instance.as_ptr()),
                    CM_LOCATE_DEVNODE_NORMAL,
                ) != CR_SUCCESS
            {
                return false;
            }
        }
        for _ in 0..12 {
            let mut buffer = [0u16; 512];
            if CM_Get_Device_IDW(node, &mut buffer, 0) == CR_SUCCESS {
                let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
                if String::from_utf16_lossy(&buffer[..end])
                    .to_ascii_uppercase()
                    .starts_with("USB\\")
                {
                    return true;
                }
            }
            let mut parent = 0;
            if CM_Get_Parent(&mut parent, node, 0) != CR_SUCCESS {
                break;
            }
            node = parent;
        }
    }
    false
}
fn manager_text(
    manager: &IPortableDeviceManager,
    id: PCWSTR,
    manufacturer: bool,
) -> Option<String> {
    let mut buffer = [0u16; 512];
    let mut length = buffer.len() as u32;
    let status = unsafe {
        if manufacturer {
            manager.GetDeviceManufacturer(id, PWSTR(buffer.as_mut_ptr()), &mut length)
        } else {
            manager.GetDeviceFriendlyName(id, PWSTR(buffer.as_mut_ptr()), &mut length)
        }
    };
    status.ok()?;
    let end = buffer.iter().position(|c| *c == 0)?;
    clean(&String::from_utf16_lossy(&buffer[..end]))
}
impl DeviceAdapter for WindowsPortableDeviceAdapter {
    fn enumerate(&self, budget: &mut Budget) -> Result<Vec<RawDevice>> {
        let _com = budget.operation("initialize_com", Apartment::new)?;
        unsafe {
            let manager: IPortableDeviceManager = budget.operation("create_manager", || {
                CoCreateInstance(&PortableDeviceManager, None, CLSCTX_INPROC_SERVER).map_err(native)
            })?;
            budget.check()?;
            budget.operation("refresh_devices", || {
                manager.RefreshDeviceList().map_err(native)
            })?;
            let mut count = 0;
            budget.operation("enumerate_devices", || {
                manager
                    .GetDevices(std::ptr::null_mut(), &mut count)
                    .map_err(native)
            })?;
            if count as usize > MAX_DEVICES {
                return Err(error("limit"));
            }
            if count == 0 {
                return Ok(vec![]);
            }
            let mut pointers = vec![PWSTR::null(); count as usize];
            let result = budget.operation("enumerate_devices", || {
                manager
                    .GetDevices(pointers.as_mut_ptr(), &mut count)
                    .map_err(native)
            });
            // Even partial/error results own allocated strings and must be freed.
            let allocated: Vec<_> = pointers.into_iter().map(Allocated).collect();
            result?;
            let mut devices = vec![];
            for p in allocated.iter().take(count as usize) {
                budget.check()?;
                let key = p.text()?;
                let id = PCWSTR(p.0.0);
                devices.push(RawDevice {
                    usb: usb_ancestor(&key),
                    key,
                    name: manager_text(&manager, id, false),
                    manufacturer: manager_text(&manager, id, true),
                });
            }
            budget.check()?;
            Ok(devices)
        }
    }
    fn open(&self, key: &str, budget: &mut Budget) -> Result<Box<dyn Content>> {
        let com = budget.operation("initialize_com", Apartment::new)?;
        budget.check()?;
        if !budget.operation("check_usb_transport", || Ok(usb_ancestor(key)))? {
            return Err(error("unsupported"));
        }
        unsafe {
            let info: IPortableDeviceValues =
                CoCreateInstance(&PortableDeviceValues, None, CLSCTX_INPROC_SERVER)
                    .map_err(native)?;
            info.SetUnsignedIntegerValue(&WPD_CLIENT_DESIRED_ACCESS, 0x80000000)
                .map_err(native)?; // GENERIC_READ, never default READWRITE.
            info.SetStringValue(&WPD_CLIENT_NAME, w!("ChatVault read-only discovery"))
                .map_err(native)?;
            let device: IPortableDevice =
                CoCreateInstance(&PortableDeviceFTM, None, CLSCTX_INPROC_SERVER).map_err(native)?;
            let id = wide(key);
            budget.operation("open_device", || {
                device.Open(PCWSTR(id.as_ptr()), &info).map_err(native)
            })?;
            let result = (|| {
                budget.check()?;
                let content =
                    budget.operation("open_content", || device.Content().map_err(native))?;
                let properties =
                    budget.operation("open_properties", || content.Properties().map_err(native))?;
                let requested =
                    keys(&[WPD_DEVICE_TYPE, WPD_DEVICE_PROTOCOL, WPD_DEVICE_TRANSPORT])?;
                let values = budget.operation("device_properties", || {
                    properties
                        .GetValues(WPD_DEVICE_OBJECT_ID, &requested)
                        .map_err(native)
                })?;
                let transport = values.GetUnsignedIntegerValue(&WPD_DEVICE_TRANSPORT).ok();
                if transport.is_some_and(|v| v != WPD_DEVICE_TRANSPORT_USB.0 as u32 && v != 0) {
                    return Err(error("unsupported"));
                }
                let category = match values.GetUnsignedIntegerValue(&WPD_DEVICE_TYPE).ok() {
                    Some(3) => Some("Phone".to_owned()),
                    Some(1) => Some("Camera".to_owned()),
                    Some(2) => Some("Media player".to_owned()),
                    Some(6) => Some("Audio recorder".to_owned()),
                    _ => None,
                };
                Ok((
                    content,
                    properties,
                    Identity {
                        category,
                        protocol: string(&values, &WPD_DEVICE_PROTOCOL),
                    },
                ))
            })();
            match result {
                Ok((content, properties, identity)) => Ok(Box::new(WpdContent {
                    properties,
                    content,
                    device,
                    identity,
                    trace: budget.trace.clone(),
                    _com: com,
                })),
                Err(e) => {
                    let _ = device.Close();
                    Err(e)
                }
            }
        }
    }
}
// Declaration order releases child interfaces before device, then apartment.
struct WpdContent {
    properties: IPortableDeviceProperties,
    content: IPortableDeviceContent,
    device: IPortableDevice,
    identity: Identity,
    trace: Option<super::scan::ScanTrace>,
    _com: Apartment,
}
impl Drop for WpdContent {
    fn drop(&mut self) {
        unsafe {
            if let Some(trace) = &self.trace {
                trace.begin("close_device");
            }
            let result = self.device.Close().map_err(native);
            if let Some(trace) = &self.trace {
                trace.end(result.as_ref().err());
            }
        }
    }
}
impl Content for WpdContent {
    fn file_metadata(&self, id: &str, budget: &mut Budget) -> Result<FileMetadata> {
        budget.check()?;
        unsafe {
            let id = wide(id);
            let requested = keys(&[
                WPD_OBJECT_ORIGINAL_FILE_NAME,
                WPD_OBJECT_NAME,
                WPD_OBJECT_SIZE,
            ])?;
            let values = self
                .properties
                .GetValues(PCWSTR(id.as_ptr()), &requested)
                .map_err(native)?;
            let name = original_string(&values, &WPD_OBJECT_ORIGINAL_FILE_NAME)
                .or_else(|| original_string(&values, &WPD_OBJECT_NAME))
                .unwrap_or_else(|| "unnamed".into());
            let size = values.GetUnsignedLargeIntegerValue(&WPD_OBJECT_SIZE).ok();
            budget.check()?;
            Ok(FileMetadata { name, size })
        }
    }
    fn read_stream(&self, id: &str, budget: &mut Budget) -> Result<Box<dyn DeviceRead>> {
        budget.check()?;
        unsafe {
            let id = wide(id);
            let resources = self.content.Transfer().map_err(native)?;
            let mut optimal = 0;
            let mut stream = None;
            resources
                .GetStream(
                    PCWSTR(id.as_ptr()),
                    &WPD_RESOURCE_DEFAULT,
                    STGM_READ.0,
                    &mut optimal,
                    &mut stream,
                )
                .map_err(native)?;
            let stream = stream.ok_or_else(|| error("unsupported"))?;
            budget.check()?;
            Ok(Box::new(WpdRead {
                stream,
                optimal: (optimal as usize).clamp(16 * 1024, 512 * 1024),
            }))
        }
    }
    fn visit_children(
        &self,
        parent: &str,
        budget: &mut Budget,
        visitor: &mut dyn FnMut(Object) -> Result<()>,
    ) -> Result<()> {
        budget.check()?;
        unsafe {
            let parent = wide(parent);
            let enumerator = budget.operation("enum_objects", || {
                self.content
                    .EnumObjects(0, PCWSTR(parent.as_ptr()), None)
                    .map_err(native)
            })?;
            let requested = keys(&[
                WPD_OBJECT_NAME,
                WPD_OBJECT_ORIGINAL_FILE_NAME,
                WPD_OBJECT_CONTENT_TYPE,
                WPD_FUNCTIONAL_OBJECT_CATEGORY,
                WPD_OBJECT_SIZE,
            ])?;
            let mut seen = std::collections::HashSet::new();
            loop {
                budget.check()?;
                let mut pointers = [PWSTR::null(); 32];
                let mut fetched = 0;
                let status = budget.operation("next_objects", || {
                    let status = enumerator.Next(&mut pointers, &mut fetched);
                    status.ok().map(|_| status).map_err(native)
                });
                let allocated: Vec<_> = pointers.into_iter().map(Allocated).collect();
                let status = status?;
                if fetched as usize > allocated.len() {
                    return Err(error("failed"));
                }
                if fetched == 0 {
                    if status.0 != 1 {
                        return Err(error("failed"));
                    } // Only S_FALSE + zero is documented EOF.
                    break;
                }
                for pointer in allocated.iter().take(fetched as usize) {
                    budget.entry()?;
                    let id = pointer.text()?;
                    if !seen.insert(id.clone()) {
                        return Err(error("duplicate_object"));
                    }
                    let values = budget.operation("object_properties", || {
                        self.properties
                            .GetValues(PCWSTR(pointer.0.0), &requested)
                            .map_err(native)
                    })?;
                    let name = original_string(&values, &WPD_OBJECT_ORIGINAL_FILE_NAME)
                        .or_else(|| original_string(&values, &WPD_OBJECT_NAME))
                        .unwrap_or_else(|| "unnamed".into());
                    let folder = values.GetGuidValue(&WPD_OBJECT_CONTENT_TYPE).ok()
                        == Some(WPD_CONTENT_TYPE_FOLDER);
                    let storage = values.GetGuidValue(&WPD_FUNCTIONAL_OBJECT_CATEGORY).ok()
                        == Some(WPD_FUNCTIONAL_CATEGORY_STORAGE);
                    if let Some(trace) = &budget.trace {
                        trace.object(folder || storage);
                    }
                    let metadata = (!folder && !storage).then(|| FileMetadata {
                        name: name.clone(),
                        size: values.GetUnsignedLargeIntegerValue(&WPD_OBJECT_SIZE).ok(),
                    });
                    visitor(Object {
                        metadata,
                        id,
                        name,
                        folder: values.GetGuidValue(&WPD_OBJECT_CONTENT_TYPE).ok()
                            == Some(WPD_CONTENT_TYPE_FOLDER),
                        storage: values.GetGuidValue(&WPD_FUNCTIONAL_OBJECT_CATEGORY).ok()
                            == Some(WPD_FUNCTIONAL_CATEGORY_STORAGE),
                    })?;
                }
            }
            budget.check()
        }
    }
    fn identity(&self) -> Identity {
        Identity {
            category: self.identity.category.clone(),
            protocol: self.identity.protocol.clone(),
        }
    }
    fn probe(&self, parent: &str, budget: &mut Budget) -> Result<()> {
        budget.check()?;
        unsafe {
            let parent = wide(parent);
            let enumerator = self
                .content
                .EnumObjects(0, PCWSTR(parent.as_ptr()), None)
                .map_err(native)?;
            let mut pointers = [PWSTR::null(); 1];
            let mut fetched = 0;
            let status = enumerator.Next(&mut pointers, &mut fetched);
            let _allocated: Vec<_> = pointers.into_iter().map(Allocated).collect();
            status.ok().map_err(native)?;
        }
        budget.check()
    }
    fn children(&self, parent: &str, budget: &mut Budget) -> Result<Vec<Object>> {
        budget.check()?;
        unsafe {
            let parent = wide(parent);
            let enumerator = self
                .content
                .EnumObjects(0, PCWSTR(parent.as_ptr()), None)
                .map_err(native)?;
            let requested = keys(&[
                WPD_OBJECT_NAME,
                WPD_OBJECT_ORIGINAL_FILE_NAME,
                WPD_OBJECT_CONTENT_TYPE,
                WPD_FUNCTIONAL_OBJECT_CATEGORY,
            ])?;
            let mut objects = vec![];
            loop {
                budget.check()?;
                let mut pointers = [PWSTR::null(); 16];
                let mut fetched = 0;
                let status = enumerator.Next(&mut pointers, &mut fetched);
                let allocated: Vec<_> = pointers.into_iter().map(Allocated).collect();
                status.ok().map_err(native)?;
                if fetched == 0 {
                    break;
                }
                for pointer in allocated.iter().take(fetched as usize) {
                    budget.entry()?;
                    if objects.len() >= MAX_CHILDREN {
                        return Err(error("limit"));
                    }
                    let id = pointer.text()?;
                    let values = self
                        .properties
                        .GetValues(PCWSTR(pointer.0.0), &requested)
                        .map_err(native)?;
                    let name = string(&values, &WPD_OBJECT_ORIGINAL_FILE_NAME)
                        .or_else(|| string(&values, &WPD_OBJECT_NAME))
                        .unwrap_or_default();
                    let folder = values.GetGuidValue(&WPD_OBJECT_CONTENT_TYPE).ok()
                        == Some(WPD_CONTENT_TYPE_FOLDER);
                    let storage = values.GetGuidValue(&WPD_FUNCTIONAL_OBJECT_CATEGORY).ok()
                        == Some(WPD_FUNCTIONAL_CATEGORY_STORAGE);
                    objects.push(Object {
                        metadata: None,
                        id,
                        name,
                        folder,
                        storage,
                    });
                }
            }
            budget.check()?;
            Ok(objects)
        }
    }
}
struct WpdRead {
    stream: IStream,
    optimal: usize,
}
impl DeviceRead for WpdRead {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut read = 0;
        unsafe {
            self.stream
                .Read(
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    Some(&mut read),
                )
                .ok()
                .map_err(native)?;
        }
        if read as usize > buffer.len() {
            return Err(error("failed"));
        }
        Ok(read as usize)
    }
    fn optimal_size(&self) -> usize {
        self.optimal
    }
}
