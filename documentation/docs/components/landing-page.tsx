import React, { useState, useEffect, useRef } from "react";
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

// ── Animation components ──

const randomRow = () => Math.floor(Math.random() * 3);
const randomPath = () => [randomRow(), randomRow(), randomRow()];

const NetworkAnimation = () => {
  // Packets traverse 5 stages: gw_e(0) → M1(1) → M2(2) → M3(3) → gw_ex(4)
  // stage -1 = not yet mounted (SSR-safe, renders all ○)
  const [packets, setPackets] = useState([
    { path: randomPath(), stage: -1 },
    { path: randomPath(), stage: -1 },
  ]);
  useEffect(() => {
    // kick off with staggered positions
    setPackets([
      { path: randomPath(), stage: 0 },
      { path: randomPath(), stage: 3 },
    ]);

    const id = setInterval(() => {
      setPackets((prev) =>
        prev.map((p) => {
          const next = (p.stage + 1) % 5;
          return { stage: next, path: next === 0 ? randomPath() : p.path };
        })
      );
    }, 300);
    return () => clearInterval(id);
  }, []);

  const gwNode = (stage: number) => {
    const active = packets.some((p) => p.stage === stage);
    return (
      <span
        style={
          active ? { color: "var(--colorPrimary)", opacity: 1 } : undefined
        }
      >
        {active ? "\u25CF" : "\u25CB"}
      </span>
    );
  };

  const mixNode = (col: number, row: number) => {
    const active = packets.some(
      (p) => p.stage === col + 1 && p.path[col] === row
    );
    const filled = active;
    return (
      <span
        style={
          active ? { color: "var(--colorPrimary)", opacity: 1 } : undefined
        }
      >
        {filled ? "\u25CF" : "\u25CB"}
      </span>
    );
  };

  return (
    <pre style={{ ...asciiStyle, marginTop: "1.2rem" }}>
      {"gw_e  M1   M2   M3  gw_ex\n"}
      {"       "}
      {mixNode(0, 0)}
      {" \u2500\u2500 "}
      {mixNode(1, 0)}
      {" \u2500\u2500 "}
      {mixNode(2, 0)}
      {"\n"}
      {"        \\  / \\  /\n"}
      {"  "}
      {gwNode(0)}
      {" \u2500\u2500 "}
      {mixNode(0, 1)}
      {" \u2500\u2500 "}
      {mixNode(1, 1)}
      {" \u2500\u2500 "}
      {mixNode(2, 1)}
      {" \u2500\u2500 "}
      {gwNode(4)}
      {"\n"}
      {"        /  \\ /  \\\n"}
      {"       "}
      {mixNode(0, 2)}
      {" \u2500\u2500 "}
      {mixNode(1, 2)}
      {" \u2500\u2500 "}
      {mixNode(2, 2)}
    </pre>
  );
};

const TypewriterAnimation = () => {
  const text =
    "let client = MixnetClient::connect_new().await?;\n" +
    "\n" +
    "client.send(msg).await;";
  const [charCount, setCharCount] = useState(0);
  const [showCursor, setShowCursor] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const run = () => {
      setCharCount(0);
      let i = 0;
      const type = () => {
        if (cancelled) return;
        if (i <= text.length) {
          setCharCount(i);
          i++;
          setTimeout(type, 40);
        } else {
          setTimeout(() => {
            if (!cancelled) run();
          }, 2000);
        }
      };
      type();
    };
    run();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const id = setInterval(() => setShowCursor((v) => !v), 530);
    return () => clearInterval(id);
  }, []);

  return (
    <pre style={{ ...asciiStyle, marginTop: "1.2rem" }}>
      {text.slice(0, charCount)}
      <span style={{ opacity: 0.6 }}>{showCursor ? "\u258C" : " "}</span>
      <span style={{ opacity: 0 }}>{text.slice(charCount)}</span>
    </pre>
  );
};

const OperatorsAnimation = () => {
  const totalBars = 10;
  const [tick, setTick] = useState(0);
  const mixRef = useRef(0);
  const [mixCount, setMixCount] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setTick((t) => t + 1);
      mixRef.current += Math.floor(Math.random() * 8) + 5;
      setMixCount(mixRef.current);
    }, 80);
    return () => clearInterval(id);
  }, []);

  const mixFilled = Math.min(tick % 12, totalBars);
  const bar = (f: number) =>
    "\u25A0".repeat(f) + "\u25A1".repeat(totalBars - f);
  const fmt = (n: number) => n.toLocaleString("en");

  return (
    <pre style={{ ...asciiStyle, marginTop: "1.2rem" }}>
      {"> nym-node run\n\n"}
      {"  mixing: "}
      {bar(mixFilled)}
      {"  "}
      {fmt(mixCount)}
      {" pkts"}
    </pre>
  );
};

const ApiAnimation = () => {
  const lines = [
    "GET /v1/mixnodes/active",
    "",
    '{ "count": 498,',
    '  "nodes": [ ... ] }',
  ];
  const [visibleLines, setVisibleLines] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const run = () => {
      setVisibleLines(0);
      setTimeout(() => {
        if (cancelled) return;
        setVisibleLines(1);
        setTimeout(() => {
          if (cancelled) return;
          let i = 2;
          const reveal = () => {
            if (cancelled) return;
            if (i <= lines.length) {
              setVisibleLines(i);
              i++;
              setTimeout(reveal, 300);
            } else {
              setTimeout(() => {
                if (!cancelled) run();
              }, 2000);
            }
          };
          reveal();
        }, 800);
      }, 100);
    };
    run();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <pre style={{ ...asciiStyle, marginTop: "1.2rem" }}>
      {lines.slice(0, visibleLines).map((line, i) => (
        <React.Fragment key={i}>
          {i > 0 && "\n"}
          {line}
        </React.Fragment>
      ))}
      <span style={{ opacity: 0 }}>
        {lines.slice(visibleLines).map((line, i) => (
          <React.Fragment key={i}>
            {visibleLines > 0 || i > 0 ? "\n" : ""}
            {line}
          </React.Fragment>
        ))}
      </span>
    </pre>
  );
};

// ── Section data ──

const sections = [
  {
    title: "Network",
    description:
      "Architecture, cryptographic systems, and how the Mixnet protects your traffic.",
    href: "/network",
    animation: "network" as const,
  },
  {
    title: "Developers",
    description: "SDKs, tutorials, and integration guides for building on Nym.",
    href: "/developers",
    animation: "typewriter" as const,
  },
  {
    title: "Operators",
    description:
      "Set up and maintain mix nodes, gateways, and network infrastructure.",
    href: "/operators/introduction",
    animation: "progress" as const,
  },
  {
    title: "APIs",
    description: "Interactive specs for querying Nym infrastructure.",
    href: "/apis/introduction",
    animation: "api" as const,
  },
];

const AnimationBlock = ({ type }: { type: string }) => {
  switch (type) {
    case "network":
      return <NetworkAnimation />;
    case "typewriter":
      return <TypewriterAnimation />;
    case "progress":
      return <OperatorsAnimation />;
    case "api":
      return <ApiAnimation />;
    default:
      return null;
  }
};

const sdks = [
  {
    name: "Rust",
    description:
      "Native SDK with async Mixnet client, streams, and TcpProxy modules.",
    href: "/developers/rust",
  },
  {
    name: "TypeScript",
    description:
      "Browser-based SDK with fetch API replacement and message-based WebSocket transport.",
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
          border: "1px solid var(--border)",
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
                borderBottom: i < 2 ? "1px solid var(--border)" : undefined,
                borderRight:
                  i % 2 === 0 ? "1px solid var(--border)" : undefined,
                display: "flex",
                flexDirection: "column",
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
                    className="landing-heading"
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "1.25rem",
                      fontWeight: 600,
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
              <AnimationBlock type={s.animation} />
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
            className="landing-heading"
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: "1.35rem",
              fontWeight: 600,
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
                  border: "1px solid var(--border)",
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
                    className="landing-heading"
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: "1rem",
                      fontWeight: 600,
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
                <span style={{ color: "var(--textMuted)", fontSize: "1rem" }}>
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
          borderTop: "1px solid var(--border)",
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
          href="https://nymtech.net"
          target="_blank"
          rel="noopener noreferrer"
          style={{ color: "var(--textMuted)", textDecoration: "none" }}
        >
          nymtech.net
        </a>
      </div>
    </div>
  );
};
