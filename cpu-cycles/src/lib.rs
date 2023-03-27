#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod bindings;
use bindings as c;

use std::fmt;
use std::{
    error::Error,
    ffi::{CStr, CString, IntoStringError},
};

#[derive(Debug)]
pub struct CpuCyclesError {
    message: String,
}

impl fmt::Display for CpuCyclesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CpuCyclesError {}

pub fn cpucycles_tracesetup() {
    unsafe { c::cpucycles_tracesetup() }
}

pub fn cpucycles() -> Result<i64, CpuCyclesError> {
    if let Some(count) = unsafe { c::cpucycles.map(|f| f()) } {
        Ok(count)
    } else {
        Err(CpuCyclesError {
            message: "Could not execute cpucycles!".to_string(),
        })
    }
}

pub fn cpucycles_persecond() -> Result<i64, CpuCyclesError> {
    Ok(unsafe { c::cpucycles_persecond() })
}

pub fn cpucycles_implementation() -> Result<String, IntoStringError> {
    let implementation = unsafe { CString::from(CStr::from_ptr(c::cpucycles_implementation())) };
    implementation.into_string()
}

pub fn cpucycles_version() -> Result<String, IntoStringError> {
    let version = unsafe { CString::from(CStr::from_ptr(c::cpucycles_version())) };
    version.into_string()
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
