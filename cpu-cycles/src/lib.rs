#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[allow(dead_code)]
mod c {
    include!("bindings.rs");
}

use std::ffi::{CStr, CString, IntoStringError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CpuCyclesError {
    #[error("Could not get cpu cycle count!")]
    CpuCycles,
    #[error("{source}")]
    StringError { source: IntoStringError },
}

pub fn cpucycles() -> Result<i64, CpuCyclesError> {
    if let Some(count) = unsafe { c::cpucycles.map(|f| f()) } {
        Ok(count)
    } else {
        Err(CpuCyclesError::CpuCycles)
    }
}

pub fn cpucycles_persecond() -> Result<i64, CpuCyclesError> {
    Ok(unsafe { c::cpucycles_persecond() })
}

pub fn cpucycles_implementation() -> Result<String, IntoStringError> {
    let implementation = unsafe { CString::from(CStr::from_ptr(c::cpucycles_implementation())) };
    Ok(implementation.into_string()?)
}

pub fn cpucycles_version() -> Result<String, IntoStringError> {
    let implementation = unsafe { CString::from(CStr::from_ptr(c::cpucycles_version())) };
    Ok(implementation.into_string()?)
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn cpucycles_test() {
        let count = cpucycles();
        assert!(count.is_ok())
    }

    #[test]
    fn cpucycles_persecond_test() {
        let per_second = cpucycles_persecond();
        assert!(per_second.is_ok());
    }

    #[test]
    fn cpucycles_implementation_test() {
        let implementation = cpucycles_implementation();
        assert!(implementation.is_ok());
    }

    #[test]
    fn cpucycles_version_test() {
        let version = cpucycles_version();
        assert!(version.is_ok());
    }
}
