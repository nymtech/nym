// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::RocketErrorResponse;
use okapi::openapi3::{OpenApi, Responses};
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Route};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::settings::OpenApiSettings;
use rocket_okapi::util::ensure_status_code_exists;
use rocket_okapi::{openapi, openapi_get_routes_spec};

// code taken from https://github.dev/GreptimeTeam/greptimedb/blob/develop/src/cmd/src/bin/greptime.rs

#[cfg(feature = "memory-prof")]
pub mod memory_prof {
    const PROF_DUMP: &[u8] = b"prof.dump\0";
    // const OPT_PROF: &[u8] = b"opt.prof\0";

    use anyhow::{bail, Context};
    use nym_config::{must_get_home, DEFAULT_NYM_APIS_DIR, NYM_DIR};
    use std::ffi::{c_char, CString};
    use time::OffsetDateTime;
    use tokio::fs::create_dir_all;
    use tokio::io::AsyncReadExt;

    pub async fn dump_profile() -> anyhow::Result<Vec<u8>> {
        if !is_prof_enabled()? {
            bail!("memory profiling is not enabled")
        }

        let now = OffsetDateTime::now_utc();
        let dump_path = must_get_home()
            .join(NYM_DIR)
            .join(DEFAULT_NYM_APIS_DIR)
            .join("memory_dumps")
            .join(format!("{}", now.unix_timestamp()))
            .join("nym-api.hprof");

        let parent = dump_path.parent().unwrap();
        create_dir_all(&parent).await?;

        info!("using {} for the memory dump", dump_path.display());

        let path = dump_path
            .to_str()
            .context("the temp dir contained invalid characters")?
            .to_string();

        let mut bytes = CString::new(path.as_str())
            .context("could not construct a CString out of the path")?
            .into_bytes_with_nul();

        {
            // #safety: we always expect a valid temp file path to write profiling data to.
            let ptr = bytes.as_mut_ptr() as *mut c_char;
            unsafe {
                tikv_jemalloc_ctl::raw::write(PROF_DUMP, ptr).context(format!(
                    "failed to dump profiling data to {}",
                    dump_path.display()
                ))?
            }
        }

        let mut f = tokio::fs::File::open(path.as_str())
            .await
            .context("failed to open the dump file")?;
        let mut buf = vec![];
        let _ = f
            .read_to_end(&mut buf)
            .await
            .context("failed to read the dump file")?;
        Ok(buf)
    }

    fn is_prof_enabled() -> anyhow::Result<bool> {
        Ok(tikv_jemalloc_ctl::profiling::prof::read()?)
        // Ok(unsafe {
        //     tikv_jemalloc_ctl::raw::read::<bool>(OPT_PROF)
        //         .context("failed to check the OPT_PROF")?
        // })
    }
}

pub struct BinaryResponse {
    inner: Vec<u8>,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for BinaryResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'o> {
        let mut res = rocket::Response::new();
        res.set_sized_body(self.inner.len(), std::io::Cursor::new(self.inner));
        Ok(res)
    }
}

impl OpenApiResponderInner for BinaryResponse {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 200);
        Ok(responses)
    }
}

/// foomp
#[cfg(feature = "memory-prof")]
#[openapi(tag = "profiling")]
#[get("/mem")]
pub async fn mem_prof_handler() -> Result<BinaryResponse, RocketErrorResponse> {
    let dump_data = memory_prof::dump_profile()
        .await
        .map_err(|err| RocketErrorResponse::new(err.to_string(), Status::InternalServerError))?;

    Ok(BinaryResponse { inner: dump_data })
}

#[cfg(not(feature = "memory-prof"))]
#[openapi(tag = "profiling")]
#[get("/mem")]
pub async fn mem_prof_handler() -> RocketErrorResponse {
    RocketErrorResponse::new("The 'mem-prof' feature is disabled", Status::NotImplemented)
}

pub(crate) fn api_status_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings:
        mem_prof_handler
    ]
}
