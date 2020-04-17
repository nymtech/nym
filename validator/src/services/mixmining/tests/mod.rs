#[cfg(test)]
pub fn fake_mixnode(location: &str) -> super::Mixnode {
    super::Mixnode {
        host: String::from("foo.com"),
        last_seen: 123,
        location: String::from(location),
        public_key: String::from("abc123"),
        stake: 8,
        version: String::from("1.0"),
    }
}
