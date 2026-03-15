import React from "react";
import Link from "next/link";

const asciiStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "0.72rem",
  lineHeight: 1.4,
  color: "var(--colorPrimary)",
  opacity: 0.7,
  whiteSpace: "pre",
  margin: 0,
};

const sections = [
  {
    title: "Network",
    description:
      "Architecture, cryptographic systems, and how the Mixnet protects your traffic.",
    href: "/network",
    ascii:
"         ●───●\n" +
"        / \\ / \\\n" +
"client → ●   ●   ● → service\n" +
"        \\ / \\ /\n" +
"         ●───●",
  },
  {
    title: "Developers",
    description: "SDKs, tutorials, and integration guides for building on Nym.",
    href: "/developers",
    ascii:
"let client = MixnetClient::connect_new().await?;\n" +
"\n" +
"client.send(msg).await;",
  },
  {
    title: "Operators",
    description:
      "Set up and maintain mix nodes, gateways, and network infrastructure.",
    href: "/operators/introduction",
    ascii:
"> nym-node run\n" +
"\n" +
"  mixing ...\n" +
"  ■■■■■■■■□□  847 packets/s",
  },
  {
    title: "APIs",
    description: "Interactive specs for querying Nym infrastructure.",
    href: "/apis/introduction",
    ascii:
"GET /v1/mixnodes/active\n" +
"\n" +
'{ "count": 498,\n' +
'  "nodes": [ ... ] }',
  },
];

const sdks = [
  {
    name: "Rust",
    description:
      "Full-featured SDK with async Mixnet client, streams, and TcpProxy.",
    href: "/developers/rust",
  },
  {
    name: "TypeScript",
    description:
      "Browser and Node.js SDK with mix-fetch and WebSocket transport.",
    href: "/developers/typescript",
  },
];


export const LandingPage = () => {
  return (
    <div
      style={{ maxWidth: "64rem", margin: "0 auto", padding: "3rem 1.5rem" }}
    >
      {/* ── Section cards ── */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(2, 1fr)",
          border: "1px solid var(--border-dark)",
          marginBottom: "3.5rem",
        }}
      >
        {sections.map((s, i) => (
          <Link
            key={i}
            href={s.href}
            style={{ textDecoration: "none", display: "flex" }}
          >
            <div
              style={{
                padding: "1.5rem",
                borderBottom:
                  i < 2 ? "1px solid var(--border-dark)" : undefined,
                borderRight:
                  i % 2 === 0 ? "1px solid var(--border-dark)" : undefined,
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
                flex: 1,
                transition: "background-color 0.15s",
                cursor: "pointer",
              }}
              className="landing-card"
            >
              <div>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "0.5rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  <h2
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "1.1rem",
                      fontWeight: 600,
                      color: "#FFFFFF",
                      margin: 0,
                      padding: 0,
                      border: "none",
                    }}
                  >
                    {s.title}
                  </h2>
                  <span
                    style={{ color: "var(--textMuted)", fontSize: "0.9rem" }}
                  >
                    &rsaquo;
                  </span>
                </div>
                <p
                  style={{
                    fontSize: "0.88rem",
                    color: "var(--textMuted)",
                    lineHeight: 1.6,
                    margin: 0,
                  }}
                >
                  {s.description}
                </p>
              </div>
              <pre style={{ ...asciiStyle, marginTop: "1.2rem" }}>
                {s.ascii}
              </pre>
            </div>
          </Link>
        ))}
      </div>

      {/* ── SDKs ── */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "0",
          marginBottom: "3.5rem",
        }}
      >
        <div style={{ paddingRight: "2rem" }}>
          <h2
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: "1.2rem",
              fontWeight: 600,
              color: "#FFFFFF",
              marginBottom: "0.5rem",
              border: "none",
              padding: 0,
            }}
          >
            SDKs
          </h2>
          <p
            style={{
              fontSize: "0.88rem",
              color: "var(--textMuted)",
              lineHeight: 1.6,
            }}
          >
            Integrate Mixnet privacy into your application with our Rust and
            TypeScript SDKs.
          </p>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "0" }}>
          {sdks.map((sdk, i) => (
            <Link key={i} href={sdk.href} style={{ textDecoration: "none" }}>
              <div
                className="landing-card"
                style={{
                  padding: "1rem 1.2rem",
                  border: "1px solid var(--border-dark)",
                  marginTop: i > 0 ? "-1px" : undefined,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  transition: "background-color 0.15s",
                  cursor: "pointer",
                }}
              >
                <div>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "0.9rem",
                      fontWeight: 600,
                      color: "#FFFFFF",
                    }}
                  >
                    {sdk.name}
                  </span>
                  <p
                    style={{
                      fontSize: "0.8rem",
                      color: "var(--textMuted)",
                      margin: "0.25rem 0 0 0",
                    }}
                  >
                    {sdk.description}
                  </p>
                </div>
                <span
                  style={{ color: "var(--textMuted)", fontSize: "1rem" }}
                >
                  &rsaquo;
                </span>
              </div>
            </Link>
          ))}
        </div>
      </div>

      {/* ── Links ── */}
      <div
        style={{
          borderTop: "1px solid var(--border-dark)",
          paddingTop: "1.5rem",
          display: "flex",
          gap: "2rem",
          fontSize: "0.82rem",
          fontFamily: "var(--font-mono)",
        }}
      >
        <a
          href="https://github.com/nymtech"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: "var(--textMuted)", textDecoration: "none" }}
        >
          GitHub
        </a>
        <a
          href="https://matrix.to/#/%23dev:nymtech.net"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: "var(--textMuted)", textDecoration: "none" }}
        >
          Matrix
        </a>
        <a
          href="https://nym.com"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: "var(--textMuted)", textDecoration: "none" }}
        >
          nym.com
        </a>
      </div>
    </div>
  );
};
