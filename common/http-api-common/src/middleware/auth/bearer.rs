// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::Request, response::Response};
use futures::future::BoxFuture;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::{debug, instrument, trace};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct BearerAuthLayer {
    bearer_token: Arc<Zeroizing<String>>,
    allow_empty: bool,
}

impl BearerAuthLayer {
    pub fn new(bearer_token: Arc<Zeroizing<String>>) -> Self {
        BearerAuthLayer {
            bearer_token,
            allow_empty: false,
        }
    }

    pub fn new_raw(bearer_token: impl Into<String>) -> Self {
        BearerAuthLayer::new(Arc::new(Zeroizing::new(bearer_token.into())))
    }

    #[must_use]
    pub fn with_allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty = allow_empty;
        self
    }
}

impl<S> Layer<S> for BearerAuthLayer {
    type Service = RequireBearerAuth<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireBearerAuth::new(inner, self.bearer_token.clone()).with_allow_empty(self.allow_empty)
    }
}

#[derive(Debug, Clone)]
pub struct RequireBearerAuth<S> {
    inner: S,
    bearer_token: Arc<Zeroizing<String>>,
    allow_empty: bool,
}

impl<S> RequireBearerAuth<S> {
    pub fn new(inner: S, bearer_token: Arc<Zeroizing<String>>) -> Self {
        RequireBearerAuth {
            inner,
            bearer_token,
            allow_empty: false,
        }
    }

    #[must_use]
    pub fn with_allow_empty(mut self, allow_empty: bool) -> Self {
        self.allow_empty = allow_empty;
        self
    }

    fn check_auth_header(&self, header: Option<&HeaderValue>) -> Result<(), &'static str> {
        let Some(token) = header else {
            trace!("missing header");
            return Err("`Authorization` header is missing");
        };

        let Ok(authorization) = token.to_str() else {
            trace!("invalid header");
            return Err("`Authorization` header contains invalid characters");
        };

        debug!("header value: '{authorization}'");

        let split = authorization.split_once(' ');
        let bearer_token = match split {
            // Found proper bearer
            Some(("Bearer", contents)) => contents,
            // Found empty bearer;
            _ if authorization == "Bearer" => "",
            // Found nothing
            _ => return Err("`Authorization` header must be a bearer token"),
        };

        debug!("parsed token: '{bearer_token}'");

        if self.bearer_token.is_empty() && bearer_token.is_empty() {
            return Ok(());
        }
        if bearer_token.is_empty() {
            return Err("`Authorization` header must contain non-empty `Bearer` token");
        }

        if self.bearer_token.as_str() != bearer_token {
            return Err("`Authorization` header does not contain the correct `Bearer` token");
        }

        Ok(())
    }
}

impl<S> Service<Request> for RequireBearerAuth<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S: Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[instrument(skip_all, fields(uri = %req.uri()))]
    fn call(&mut self, req: Request) -> Self::Future {
        debug!("checking the auth");

        if self.bearer_token.is_empty() && !self.allow_empty {
            return Box::pin(async move {
                Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no valid access token has been specified on the server",
                )
                    .into_response())
            });
        }

        let auth_header = req.headers().get(header::AUTHORIZATION);

        match self.check_auth_header(auth_header) {
            Ok(_authorised) => Box::pin(self.inner.call(req)),
            Err(err) => {
                Box::pin(async move { Ok((StatusCode::UNAUTHORIZED, err).into_response()) })
            }
        }
    }
}
