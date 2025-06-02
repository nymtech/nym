// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{io, path::Path};

use proc_pidinfo::{
    ProcFDInfo, ProcFDType, VnodeFdInfoWithPath, proc_pidfdinfo_self, proc_pidinfo_list_self,
};

/// Check if there are no open file descriptors for the given files.
///
/// Uses `proc_pidinfo` (`sys/proc_info.h`)
/// See: http://blog.palominolabs.com/2012/06/19/getting-the-files-being-used-by-a-process-on-mac-os-x/
pub async fn check_files_closed(file_paths: &[&Path]) -> io::Result<bool> {
    let fd_list = proc_pidinfo_list_self::<ProcFDInfo>()?;

    for fd in fd_list
        .iter()
        .filter(|s| s.fd_type() == Ok(ProcFDType::VNODE))
    {
        let Some(vnode) = proc_pidfdinfo_self::<VnodeFdInfoWithPath>(fd.proc_fd)
            .inspect_err(|e| {
                log::warn!("proc_pidfdinfo_self::<VnodeFdInfoWithPath>() failure: {e}");
            })
            .ok()
            .flatten()
        else {
            continue;
        };

        if let Ok(true) = vnode
            .path()
            .map(|vnode_path| file_paths.contains(&vnode_path))
            .inspect_err(|e| {
                log::warn!("vnode.path() failure: {e:?}");
            })
        {
            return Ok(false);
        }
    }

    Ok(true)
}
