// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    ffi::{OsString, c_uchar, c_ulong, c_ushort, c_void},
    io,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
};

use windows::{
    Wdk::System::SystemInformation::{NtQuerySystemInformation, SYSTEM_INFORMATION_CLASS},
    Win32::{
        Foundation::{HANDLE, MAX_PATH, NTSTATUS, STATUS_INFO_LENGTH_MISMATCH},
        Storage::FileSystem::{
            FILE_NAME_NORMALIZED, FILE_TYPE_DISK, GetFileType, GetFinalPathNameByHandleW,
        },
        System::{
            Memory::{
                GetProcessHeap, HEAP_FLAGS, HEAP_ZERO_MEMORY, HeapAlloc, HeapFree, HeapReAlloc,
            },
            Threading::GetCurrentProcessId,
        },
    },
};

/// Private information class used to retrieve open file handles
const SYSTEM_HANDLE_INFORMATION_CLASS: SYSTEM_INFORMATION_CLASS = SYSTEM_INFORMATION_CLASS(0x10);

/// Initial buffer size holding the handle info
/// The number is based on what I observe on a pretty standard Windows 11
const SYSTEM_HANDLE_INFORMATION_INITIAL_SIZE: usize = 2_500_000;

/// Max retry attempts for querying system handle information before giving up
const MAX_RETRY_ATTEMPTS: u32 = 5;

/// Check if there are no open handles to the given files.
///
/// Uses undocumented NT API to obtain open handles on the system.
/// See: https://www.ired.team/miscellaneous-reversing-forensics/windows-kernel-internals/get-all-open-handles-and-kernel-object-address-from-userland
pub async fn check_files_closed(file_paths: &[&Path]) -> io::Result<bool> {
    let current_pid = unsafe { GetCurrentProcessId() };
    let handle_table_info = query_system_handle_table()?;

    // Convert returned data into slice
    let num_handles = unsafe { (*handle_table_info.inner).number_of_handles };
    let proc_entries = unsafe {
        std::slice::from_raw_parts(
            (*handle_table_info.as_mut_ptr()).handles.as_ptr(),
            num_handles as usize,
        )
    };

    // Iterate over open file handle entries
    for entry in proc_entries {
        if entry.unique_process_id == current_pid {
            let file_handle = HANDLE(entry.handle_value as _);

            // Filter everything except disk files
            if unsafe { GetFileType(file_handle) } == FILE_TYPE_DISK {
                // Obtain canonical path for file handle
                let mut file_handle_path = vec![0u16; MAX_PATH as usize];
                let num_chars_without_nul = unsafe {
                    GetFinalPathNameByHandleW(
                        file_handle,
                        &mut file_handle_path,
                        FILE_NAME_NORMALIZED,
                    ) as usize
                };

                if num_chars_without_nul > 0 {
                    let path_str = OsString::from_wide(&file_handle_path[0..num_chars_without_nul]);
                    let file_handle_pathbuf = PathBuf::from(path_str);
                    if file_paths.contains(&file_handle_pathbuf.as_path()) {
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

fn query_system_handle_table() -> io::Result<HeapGuard<SystemHandleInformation>> {
    // Allocate info struct on heap with some initial value
    let mut reserved_memory = SYSTEM_HANDLE_INFORMATION_INITIAL_SIZE;
    let mut handle_table_info = HeapGuard::<SystemHandleInformation>::new(reserved_memory)?;

    // Request system handle information
    let mut status: NTSTATUS = NTSTATUS::default();
    for _ in 0..MAX_RETRY_ATTEMPTS {
        let mut return_len = reserved_memory as u32;
        status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_HANDLE_INFORMATION_CLASS,
                handle_table_info.as_mut_ptr() as _,
                return_len,
                &mut return_len,
            )
        };

        // Buffer is too small, resize memory and retry again.
        if status == STATUS_INFO_LENGTH_MISMATCH {
            // Allocate a bit more memory since the size of table can change between calls
            let resize_to = (return_len as usize) * 3 / 2;

            tracing::trace!(
                "Buffer is too small ({reserved_memory}), returned length: {return_len}, resizing buffer to: {resize_to}"
            );

            reserved_memory = resize_to;
            handle_table_info.reallocate(reserved_memory)?;
        } else {
            break;
        }
    }

    Ok(status.ok().map(|_| handle_table_info)?)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SystemHandleInformation {
    pub number_of_handles: c_ulong,
    pub handles: [SystemHandleTableEntryInfo; 1],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SystemHandleTableEntryInfo {
    pub unique_process_id: c_ulong,
    pub object_type_index: c_uchar,
    pub handle_attributes: c_uchar,
    pub handle_value: c_ushort,
    pub object: *mut c_void,
    pub granted_access: c_ulong,
}

/// Managed heap memory
struct HeapGuard<T> {
    inner: *mut T,
    process_heap: HANDLE,
}

impl<T> HeapGuard<T> {
    /// Allocate new memory using `HealAlloc`
    fn new(length: usize) -> io::Result<Self> {
        let process_heap = unsafe { GetProcessHeap()? };
        let inner: *mut T = unsafe { HeapAlloc(process_heap, HEAP_ZERO_MEMORY, length) as _ };

        if inner.is_null() {
            Err(io::Error::other("Failed to allocate memory"))
        } else {
            Ok(Self {
                inner,
                process_heap,
            })
        }
    }

    /// Reallocate existing chunk of memory
    ///
    /// On success: the internal memory pointer is replaced.
    /// On failure: the internal memory pointer remains the same and still valid.
    fn reallocate(&mut self, new_length: usize) -> io::Result<()> {
        let new_ptr: *mut T = unsafe {
            HeapReAlloc(
                self.process_heap,
                HEAP_ZERO_MEMORY,
                Some(self.inner as _),
                new_length,
            ) as _
        };

        if new_ptr.is_null() {
            Err(io::Error::other("Failed to reallocate memory"))
        } else {
            self.inner = new_ptr;
            Ok(())
        }
    }

    fn as_mut_ptr(&self) -> *mut T {
        self.inner
    }
}

impl<T> Drop for HeapGuard<T> {
    fn drop(&mut self) {
        #[allow(clippy::expect_used)]
        unsafe {
            HeapFree(
                self.process_heap,
                HEAP_FLAGS(0),
                Some(self.inner as *mut c_void),
            )
        }
        .expect("HeapFree failure");
    }
}
