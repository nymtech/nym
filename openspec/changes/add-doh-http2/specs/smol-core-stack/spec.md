# smol-core-stack Spec Delta

## ADDED Requirements

### Requirement: DoH negotiates HTTP/2 where the resolver requires it

The DoH request path SHALL advertise both HTTP/2 and HTTP/1.1 by TLS ALPN and speak whichever the resolver selects. A resolver that requires HTTP/2 (for example Quad9, which retired HTTP/1.1 DoH) SHALL resolve successfully rather than returning an HTTP-version error. A resolver that offers HTTP/1.1 SHALL continue to use the existing HTTP/1.1 path with no behavioural change. A DoH connection that speaks HTTP/2 serves a single request and is not pooled; because resolutions are cached per hostname per session, the cold handshake is paid once per unique hostname, not per lookup.

#### Scenario: An HTTP/2-only resolver resolves

- **WHEN** the pinned DoH resolver selects HTTP/2 by ALPN and answers `200` with a wire-format DNS message
- **THEN** the query resolves to an IP, rather than failing with `505 HTTP Version Not Supported`

#### Scenario: An HTTP/1.1 resolver is unchanged

- **WHEN** the pinned DoH resolver selects HTTP/1.1 by ALPN
- **THEN** the request uses the existing pooled HTTP/1.1 path with the same behaviour as before this change

#### Scenario: The HTTP/2 DoH cost is paid once per hostname

- **WHEN** the same hostname is resolved twice in a session over an HTTP/2 resolver
- **THEN** the second resolution is served from the session cache without a second DoH handshake

### Requirement: The general fetch path stays HTTP/1.1-only

The general `mixFetch` TLS configuration SHALL advertise `http/1.1` only, so a website can never negotiate HTTP/2 against the HTTP/1.1-only request code. HTTP/2 SHALL be confined to the DoH request path and its own TLS configuration. This keeps the working fetch path unaffected: a website that would offer HTTP/2 still connects over HTTP/1.1.

#### Scenario: A website that offers HTTP/2 still uses HTTP/1.1

- **WHEN** `mixFetch` connects to a website whose server offers HTTP/2
- **THEN** ALPN selects `http/1.1` because that is all the fetch configuration advertises, and the request runs over HTTP/1.1 as before

#### Scenario: HTTP/2 does not leak onto the shared configuration

- **WHEN** the DoH path is built with HTTP/2 enabled
- **THEN** the general fetch TLS configuration is unchanged and still advertises `http/1.1` only
