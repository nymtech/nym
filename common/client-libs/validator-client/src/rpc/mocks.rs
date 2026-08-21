// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::Height;
use crate::rpc::TendermintRpcClient;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tendermint_rpc::endpoint::{commit, validators};
use tendermint_rpc::{Error, PageNumber, Paging, PerPage, SimpleRequest};

// reimplementation of `Paging` that derives `Hash`
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum PagingWrapper {
    Default,
    All,
    Specific {
        page_number: PageNumberWrapper,
        per_page: PerPageWrapper,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct PageNumberWrapper(usize);
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct PerPageWrapper(u8);

#[allow(clippy::unwrap_used)]
impl From<Paging> for PagingWrapper {
    fn from(value: Paging) -> Self {
        match value {
            Paging::Default => PagingWrapper::Default,
            Paging::All => PagingWrapper::All,
            Paging::Specific {
                page_number,
                per_page,
            } => PagingWrapper::Specific {
                page_number: PageNumberWrapper(page_number.to_string().parse().unwrap()),
                per_page: PerPageWrapper(per_page.to_string().parse().unwrap()),
            },
        }
    }
}

impl From<PagingWrapper> for Paging {
    fn from(value: PagingWrapper) -> Self {
        match value {
            PagingWrapper::All => Paging::All,
            PagingWrapper::Default => Paging::Default,
            PagingWrapper::Specific {
                page_number,
                per_page,
            } => Paging::Specific {
                page_number: PageNumber::from(page_number.0),
                per_page: PerPage::from(per_page.0),
            },
        }
    }
}

#[derive(Default)]
struct CallLog {
    commit: Vec<Height>,
    validators: Vec<(Height, Paging)>,
}

// very naive mock for rpc queries that currently only support tiny subset of pre-registered queries
#[derive(Clone, Default)]
pub struct MockRpcClient {
    commits: HashMap<Height, Result<commit::Response, Error>>,
    validators: HashMap<(Height, PagingWrapper), Result<validators::Response, Error>>,

    call_log: Arc<Mutex<CallLog>>,
}

impl MockRpcClient {
    /// Heights passed to `commit`, in call order.
    pub fn commit_calls(&self) -> Vec<Height> {
        self.call_log.lock().unwrap().commit.clone()
    }

    /// Heights passed to `validators`, in call order.
    pub fn validators_calls(&self) -> Vec<(Height, Paging)> {
        self.call_log.lock().unwrap().validators.clone()
    }

    pub fn with_commit_response<H>(
        &mut self,
        height: H,
        response: Result<commit::Response, Error>,
    ) -> &mut Self
    where
        H: Into<Height> + Send,
    {
        self.commits.insert(height.into(), response);
        self
    }

    pub fn with_validators_response<H>(
        &mut self,
        height: H,
        paging: Paging,
        response: Result<validators::Response, Error>,
    ) -> &mut Self
    where
        H: Into<Height> + Send,
    {
        self.validators
            .insert((height.into(), paging.into()), response);
        self
    }
}

#[async_trait]
impl TendermintRpcClient for MockRpcClient {
    async fn commit<H>(&self, height: H) -> Result<commit::Response, Error>
    where
        H: Into<Height> + Send,
    {
        let height = height.into();
        self.call_log.lock().unwrap().commit.push(height);
        self.commits
            .get(&height)
            .unwrap_or_else(|| panic!("unregistered response for commit at height {height}"))
            .clone()
    }

    async fn validators<H>(&self, height: H, paging: Paging) -> Result<validators::Response, Error>
    where
        H: Into<Height> + Send,
    {
        let height = height.into();
        self.call_log
            .lock()
            .unwrap()
            .validators
            .push((height, paging));
        self.validators
            .get(&(height, paging.into()))
            .unwrap_or_else(|| panic!("unregistered response for validators at height {height} with pagination {paging:#?}"))
            .clone()
    }

    async fn perform<R>(&self, _: R) -> Result<R::Output, Error>
    where
        R: SimpleRequest,
    {
        unimplemented!()
    }
}
