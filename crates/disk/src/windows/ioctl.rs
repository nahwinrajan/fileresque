//! Win32 IOCTL wrappers for physical disk enumeration.
//!
//! All functions in this module call OS APIs and therefore cannot be unit-tested
//! on non-Windows platforms. Integration testing on Windows is deferred to P5.
//!
//! Compiled only when `target_os = "windows"`.

#![cfg(target_os = "windows")]
// Wildcard imports are necessary here: windows-sys re-exports hundreds of
// Win32 constants and structs under deep module paths, and explicit imports
// would triple the size of this file with no clarity benefit.
#![allow(clippy::wildcard_imports)]

use windows_sys::Win32::{
    Foundation::*, Storage::FileSystem::*, System::Ioctl::*, System::IO::DeviceIoControl,
};

use fileresque_core::error::AppError;

use super::enumerate::{extract_offset_string, RawDiskInfo};

/// Enumerate all physical drives by opening `\\.\PhysicalDrive0` through
/// `\\.\PhysicalDrive15`.
///
/// Stops at the first `ERROR_FILE_NOT_FOUND` / `ERROR_PATH_NOT_FOUND` — Windows
/// assigns drive indices consecutively so a gap means no more drives exist.
///
/// # Errors
///
/// - [`AppError::PermissionDenied`] when `CreateFileW` returns `ERROR_ACCESS_DENIED`.
/// - [`AppError::Internal`] on unexpected Win32 errors.
pub(crate) fn enumerate_physical_drives() -> Result<Vec<RawDiskInfo>, AppError> {
    let mut disks = Vec::new();

    for i in 0..16u32 {
        let path = format!(r"\\.\PhysicalDrive{i}");
        // Encode to UTF-16 and append the null terminator required by CreateFileW.
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: `wide_path` is a valid null-terminated UTF-16 string whose
        // lifetime extends across the CreateFileW call. The pointer is valid for
        // the duration of this call.
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                0,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            // SAFETY: GetLastError is always safe to call immediately after a
            // failed Win32 call; it reads thread-local error state.
            let err = unsafe { GetLastError() };
            if err == ERROR_FILE_NOT_FOUND || err == ERROR_PATH_NOT_FOUND {
                // No more physical drives — stop searching.
                break;
            }
            if err == ERROR_ACCESS_DENIED {
                return Err(AppError::PermissionDenied(
                    "Disk access requires Administrator privileges".to_string(),
                ));
            }
            // Other errors (e.g. drive being initialised) — skip this index.
            continue;
        }

        match query_disk_info(handle, i) {
            Ok(raw) => disks.push(raw),
            Err(_) => {} // Skip drives whose metadata cannot be read.
        }

        // SAFETY: `handle` was returned by CreateFileW and is valid. Closing it
        // here is the only place this handle is closed; no other code holds a
        // copy.
        unsafe { CloseHandle(handle) };
    }

    Ok(disks)
}

/// Query size and storage descriptor for an already-open disk handle.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if any IOCTL call fails.
fn query_disk_info(handle: HANDLE, index: u32) -> Result<RawDiskInfo, AppError> {
    let size_bytes = query_disk_size(handle)?;
    let (friendly_name, serial, bus_type, is_removable) = query_storage_descriptor(handle)?;

    Ok(RawDiskInfo {
        device_path: format!(r"\\.\PhysicalDrive{index}"),
        friendly_name,
        serial,
        size_bytes,
        bus_type,
        is_removable,
        partition_style: 0, // Partition-style detection deferred to a future task.
    })
}

/// Issue `IOCTL_DISK_GET_LENGTH_INFO` to obtain total disk capacity.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if the IOCTL call fails.
fn query_disk_size(handle: HANDLE) -> Result<u64, AppError> {
    // SAFETY: `GET_LENGTH_INFORMATION` is a Plain Old Data type. Zero-initialising
    // it is safe and matches the pattern required by DeviceIoControl output buffers.
    let mut length_info: GET_LENGTH_INFORMATION = unsafe { std::mem::zeroed() };
    let mut bytes_returned: u32 = 0;

    // SAFETY: `handle` is open and valid. `length_info` is properly sized and
    // aligned for `GET_LENGTH_INFORMATION`. The output pointer and size are
    // consistent.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_DISK_GET_LENGTH_INFO,
            std::ptr::null(),
            0,
            std::ptr::addr_of_mut!(length_info).cast(),
            u32::try_from(std::mem::size_of::<GET_LENGTH_INFORMATION>())
                // JUSTIFIED: sizeof(GET_LENGTH_INFORMATION) is 8 bytes — well within u32.
                .expect("GET_LENGTH_INFORMATION size fits in u32"),
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(AppError::Internal(
            "IOCTL_DISK_GET_LENGTH_INFO failed".to_string(),
        ));
    }

    // SAFETY: `Length` is a LARGE_INTEGER union. We access `QuadPart` (i64)
    // which gives the full signed 64-bit length. Physical disk sizes are
    // non-negative, so casting to u64 is safe.
    let size = unsafe { length_info.Length.QuadPart } as u64;
    Ok(size)
}

/// Issue `IOCTL_STORAGE_QUERY_PROPERTY` (two-pass: header then full) to
/// read product name, serial number, bus type, and removable flag.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if either IOCTL pass fails or if the
/// returned buffer is smaller than `STORAGE_DEVICE_DESCRIPTOR`.
fn query_storage_descriptor(
    handle: HANDLE,
) -> Result<(String, Option<String>, u8, bool), AppError> {
    let mut query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0u8; 1],
    };

    // ---- Pass 1: ask for the required buffer size ----
    // SAFETY: `STORAGE_DESCRIPTOR_HEADER` is POD; zero-init is safe.
    let mut descriptor_header: STORAGE_DESCRIPTOR_HEADER = unsafe { std::mem::zeroed() };
    let mut bytes_returned: u32 = 0;

    // SAFETY: Both `query` and `descriptor_header` are valid, properly aligned,
    // and sized for their types. `handle` is open.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            std::ptr::addr_of_mut!(query).cast(),
            u32::try_from(std::mem::size_of::<STORAGE_PROPERTY_QUERY>())
                // JUSTIFIED: sizeof is a small constant that fits in u32.
                .expect("STORAGE_PROPERTY_QUERY size fits in u32"),
            std::ptr::addr_of_mut!(descriptor_header).cast(),
            u32::try_from(std::mem::size_of::<STORAGE_DESCRIPTOR_HEADER>())
                // JUSTIFIED: sizeof is a small constant that fits in u32.
                .expect("STORAGE_DESCRIPTOR_HEADER size fits in u32"),
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(AppError::Internal(
            "IOCTL_STORAGE_QUERY_PROPERTY (header pass) failed".to_string(),
        ));
    }

    let buf_size = descriptor_header.Size as usize;
    if buf_size < std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
        return Err(AppError::Internal(
            "STORAGE_DESCRIPTOR_HEADER.Size is smaller than STORAGE_DEVICE_DESCRIPTOR".to_string(),
        ));
    }

    // ---- Pass 2: fetch the full descriptor ----
    let mut buf = vec![0u8; buf_size];
    let buf_len = u32::try_from(buf_size)
        // JUSTIFIED: buf_size comes from STORAGE_DESCRIPTOR_HEADER.Size (u32)
        // cast to usize, so it always fits back into u32.
        .expect("buf_size fits in u32");

    // SAFETY: `buf` is heap-allocated with exactly `buf_size` bytes and lives
    // for the duration of this call. The DeviceIoControl output pointer and
    // length are consistent with the allocation.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            std::ptr::addr_of_mut!(query).cast(),
            u32::try_from(std::mem::size_of::<STORAGE_PROPERTY_QUERY>())
                // JUSTIFIED: sizeof is a small constant that fits in u32.
                .expect("STORAGE_PROPERTY_QUERY size fits in u32"),
            buf.as_mut_ptr().cast(),
            buf_len,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(AppError::Internal(
            "IOCTL_STORAGE_QUERY_PROPERTY (full pass) failed".to_string(),
        ));
    }

    extract_descriptor_fields(&buf)
}

/// Parse product name, serial number, bus type, and removable flag from a
/// `STORAGE_DEVICE_DESCRIPTOR` buffer.
///
/// Separated from [`query_storage_descriptor`] to keep cognitive complexity ≤ 15.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if the buffer is too small.
fn extract_descriptor_fields(buf: &[u8]) -> Result<(String, Option<String>, u8, bool), AppError> {
    if buf.len() < std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() {
        return Err(AppError::Internal(
            "Descriptor buffer too small for STORAGE_DEVICE_DESCRIPTOR".to_string(),
        ));
    }

    // SAFETY: We verified `buf.len() >= sizeof(STORAGE_DEVICE_DESCRIPTOR)`.
    // The buffer was populated by DeviceIoControl with the correct layout.
    // The reference does not outlive `buf`.
    // Alignment: STORAGE_DEVICE_DESCRIPTOR requires 4-byte alignment (widest
    // field is ULONG/u32). The Windows system heap guarantees minimum 8-byte
    // alignment on x86 and 16-byte alignment on x64 for all non-zero heap
    // allocations (Vec<u8> included), satisfying this requirement on all
    // supported targets.
    let desc = unsafe { &*(buf.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>()) };

    let friendly_name = extract_offset_string(buf, desc.ProductIdOffset as usize)
        .unwrap_or_else(|| "Unknown Disk".to_string());
    let serial = extract_offset_string(buf, desc.SerialNumberOffset as usize);
    let bus_type = desc.BusType as u8;
    let is_removable = desc.RemovableMedia != 0;

    Ok((friendly_name, serial, bus_type, is_removable))
}
