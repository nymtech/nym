import React from "react";
import { Ghost, Zap, Code, Soup } from "lucide-react";

export const LandingPage = () => {
  const squares = [
    { text: "Network Docs", href: "/network", Icon: Ghost },
    {
      text: "Developers: Core Concepts, Integration Overview, Tools & Tutorials",
      href: "/developers",
      Icon: Zap,
    },
    { text: "SDKs", href: "/sdk", Icon: Code },
    {
      text: "Operators: Setup Guides & Maintenance",
      href: "/operators",
      Icon: Soup,
    },
  ];

  return (
    <div className="landing-page-container">
      <p className="landing-page-intro">
        Nym is a privacy platform. It provides strong network-level privacy
        against sophisticated end-to-end attackers, and anonymous access control
        using blinded, re-randomizable, decentralized credentials. Our goal is
        to allow developers to build new applications, or upgrade existing apps,
        with privacy features unavailable in other systems.
      </p>

      <div className="landing-page-grid">
        {squares.map((square, index) => (
          <a key={index} href={square.href} className="landing-page-square">
            <square.Icon className="landing-page-icon" />
            <span className="landing-page-text">{square.text}</span>
          </a>
        ))}
      </div>

      <style jsx>{`
        .landing-page-container {
          max-width: 1200px;
          margin: 0 auto;
          padding: 2rem 1rem;
        }
        .landing-page-intro {
          font-size: 1.125rem;
          margin-bottom: 2rem;
        }
        .landing-page-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 2rem;
          max-width: 36rem;
          margin: 0 auto;
        }
        .landing-page-square {
          background-color: #000000;
          color: white;
          padding: 1rem;
          border-radius: 0.5rem;
          text-align: center;
          cursor: pointer;
          text-decoration: none;
          transition: box-shadow 0.3s ease;
          aspect-ratio: 1 / 1;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          border: 3px solid #ff6600;
          box-shadow: 0 0 10px #ff6600;
        }
        .landing-page-square:hover {
          box-shadow: 0 0 20px #ff6600;
        }
        .landing-page-icon {
          width: 12.5rem;
          height: 12.5rem;
          margin-bottom: 0.75rem;
          color: #ff6600;
        }
        .landing-page-text {
          font-size: 0.875rem;
          font-weight: 600;
          color: #ff6600;
        }
      `}</style>
    </div>
  );
};
