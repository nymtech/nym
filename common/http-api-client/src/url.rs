//! Url handling for the HTTP API client.
//!
//! This module provides a `Url` struct that wraps around the `url::Url` type and adds
//! functionality for handling front domains, which are used for reverse proxying.

use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use itertools::Itertools;
use url::form_urlencoded;
pub use url::ParseError;

/// A trait to try to convert some type into a `Url`.
pub trait IntoUrl {
    /// Parse as a valid `Url`
    fn to_url(self) -> Result<Url, ParseError>;

    /// Returns the string representation of the URL.
    fn as_str(&self) -> &str;
}

impl IntoUrl for &str {
    fn to_url(self) -> Result<Url, ParseError> {
        let url = url::Url::parse(self)?;
        Ok(url.into())
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl IntoUrl for String {
    fn to_url(self) -> Result<Url, ParseError> {
        let url = url::Url::parse(&self)?;
        Ok(url.into())
    }

    fn as_str(&self) -> &str {
        self
    }
}

impl IntoUrl for reqwest::Url {
    fn to_url(self) -> Result<Url, ParseError> {
        Ok(self.into())
    }

    fn as_str(&self) -> &str {
        self.as_str()
    }
}

/// When configuring fronting, some configurations will require a specific backend host
/// to be used for the request to be properly reverse proxied.
#[derive(Debug, Clone)]
pub struct Url {
    url: url::Url,
    fronts: Option<Vec<url::Url>>,
    current_front: Arc<AtomicUsize>,
}

impl IntoUrl for Url {
    fn to_url(self) -> Result<Url, ParseError> {
        Ok(self)
    }

    fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

impl PartialEq for Url {
    fn eq(&self, other: &Self) -> bool {
        let current = self.current_front.load(Ordering::Relaxed);
        let other_current = other.current_front.load(Ordering::Relaxed);

        self.fronts == other.fronts && self.url == other.url && current == other_current
    }
}

impl Eq for Url {}

impl std::hash::Hash for Url {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let current = self.current_front.load(Ordering::Relaxed);
        self.fronts.hash(state);
        self.url.hash(state);
        current.hash(state);
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.fronts {
            Some(ref fronts) => {
                let current = self.current_front.load(Ordering::Relaxed);
                if let Some(front) = fronts.get(current) {
                    write!(f, "{front}=>{}", self.url)
                } else {
                    write!(f, "{}", self.url)
                }
            }
            None => write!(f, "{}", self.url),
        }
    }
}

impl From<Url> for url::Url {
    fn from(val: Url) -> Self {
        val.url
    }
}

impl From<reqwest::Url> for Url {
    fn from(url: url::Url) -> Self {
        Self {
            url,
            fronts: None,
            current_front: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl AsRef<url::Url> for Url {
    fn as_ref(&self) -> &url::Url {
        &self.url
    }
}

impl AsMut<url::Url> for Url {
    fn as_mut(&mut self) -> &mut url::Url {
        &mut self.url
    }
}

impl std::str::FromStr for Url {
    type Err = url::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = url::Url::parse(s)?;
        Ok(Self {
            url,
            fronts: None,
            current_front: Arc::new(AtomicUsize::new(0)),
        })
    }
}

impl Url {
    /// Create a new `Url` instance with the given something that can be parsed as a  URL and
    /// optional tunneling domains
    pub fn new<U: reqwest::IntoUrl>(
        url: U,
        fronts: Option<Vec<U>>,
    ) -> Result<Self, reqwest::Error> {
        let mut url = Self {
            url: url.into_url()?,
            fronts: None,
            current_front: Arc::new(AtomicUsize::new(0)),
        };

        // ensure that the provided URLs are valid
        if let Some(front_domains) = fronts {
            let f: Vec<reqwest::Url> = front_domains
                .into_iter()
                .map(|front| front.into_url())
                .try_collect()?;
            url.fronts = Some(f);
        }

        Ok(url)
    }

    /// Parse an absolute URL from a string.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let url = url::Url::parse(s)?;
        Ok(Self {
            url,
            fronts: None,
            current_front: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Returns true if the URL has a front domain set
    pub fn has_front(&self) -> bool {
        if let Some(fronts) = &self.fronts {
            return !fronts.is_empty();
        }
        false
    }

    /// Return the string representation of the current front host (domain or IP address) for this
    /// URL, if any.
    pub fn front_str(&self) -> Option<&str> {
        let current = self.current_front.load(Ordering::Relaxed);
        self.fronts
            .as_ref()
            .and_then(|fronts| fronts.get(current))
            .and_then(|url| url.host_str())
    }

    /// Return the string representation of the host (domain or IP address) for this URL, if any.
    pub fn host_str(&self) -> Option<&str> {
        self.url.host_str()
    }

    /// Return the serialization of this URL.
    ///
    /// This is fast since that serialization is already stored in the inner url::Url struct.
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    /// Returns true if updating the front wraps back to the first front, or if no fronts are set
    pub fn update(&self) -> bool {
        if let Some(fronts) = &self.fronts {
            if fronts.len() > 1 {
                let current = self.current_front.load(Ordering::Relaxed);
                let next = (current + 1) % fronts.len();
                self.current_front.store(next, Ordering::Relaxed);
                return next == 0;
            }
        }
        true
    }

    /// Return the scheme of this URL, lower-cased, as an ASCII string without the ‘:’ delimiter.
    pub fn scheme(&self) -> &str {
        self.url.scheme()
    }

    /// Parse the URL’s query string, if any, as application/x-www-form-urlencoded and return an
    /// iterator of (key, value) pairs.
    pub fn query_pairs(&self) -> form_urlencoded::Parse<'_> {
        self.url.query_pairs()
    }

    /// Manipulate this URL’s query string, viewed as a sequence of name/value pairs in
    /// application/x-www-form-urlencoded syntax.
    pub fn query_pairs_mut(&mut self) -> form_urlencoded::Serializer<'_, ::url::UrlQuery<'_>> {
        self.url.query_pairs_mut()
    }

    /// Change this URL’s query string. If `query` is `None`, this URL’s query string will be cleared.
    pub fn set_query(&mut self, query: Option<&str>) {
        self.url.set_query(query);
    }

    /// Change this URL’s path.
    pub fn set_path(&mut self, path: &str) {
        self.url.set_path(path);
    }

    /// Change this URL’s scheme.
    pub fn set_scheme(&mut self, scheme: &str) {
        self.url.set_scheme(scheme).unwrap();
    }

    /// Change this URL’s host.
    ///
    /// Removing the host (calling this with None) will also remove any username, password, and port number.
    pub fn set_host(&mut self, host: &str) {
        self.url.set_host(Some(host)).unwrap();
    }

    /// Change this URL’s port number.
    ///
    /// Note that default port numbers are not reflected in the serialization.
    ///
    /// If this URL is cannot-be-a-base, does not have a host, or has the `file` scheme; do nothing and return `Err`.
    pub fn set_port(&mut self, port: u16) {
        self.url.set_port(Some(port)).unwrap();
    }

    /// Return an object with methods to manipulate this URL’s path segments.
    ///
    /// Return Err(()) if this URL is cannot-be-a-base.
    pub fn path_segments(&self) -> Option<std::str::Split<'_, char>> {
        self.url.path_segments()
    }

    /// Return an object with methods to manipulate this URL’s path segments.
    ///
    /// Return Err(()) if this URL is cannot-be-a-base.
    #[allow(clippy::result_unit_err)]
    pub fn path_segments_mut(&mut self) -> Result<::url::PathSegmentsMut<'_>, ()> {
        self.url.path_segments_mut()
    }
}
