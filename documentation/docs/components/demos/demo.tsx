"use client";

// `<Demo name="EnsDemo" />`: an interactive demo in a snippet. A snippet imports
// nothing, so it names the demo, and mdx-components.jsx (part of every page)
// holds only this small registry: each demo loads as its own chunk, only on a
// page that shows it. Other props pass through (`<Demo name="PacketAnatomy" mode="dvpn" />`).
// The demos that load WASM or use browser APIs render on the client only, as
// they did before; the playground renders on the server too.
import dynamic from "next/dynamic";
import type { ComponentType } from "react";

// next/dynamic needs its options as an object literal at each call.
const DEMOS: Record<string, ComponentType<Record<string, unknown>>> = {
  DvpnCoverTraffic: dynamic(() => import("../threat-model/DvpnCoverTraffic").then((m) => m.DvpnCoverTraffic), { ssr: false }),
  MixnetDeepDive: dynamic(() => import("../threat-model/MixnetDeepDive").then((m) => m.MixnetDeepDive), { ssr: false }),
  PacketAnatomy: dynamic(() => import("../threat-model/PacketAnatomy").then((m) => m.PacketAnatomy), { ssr: false }),
  SurbDirectory: dynamic(() => import("../threat-model/SurbDirectory").then((m) => m.SurbDirectory), { ssr: false }),
  EnsDemo: dynamic(() => import("./ens/EnsDemo").then((m) => m.EnsDemo), { ssr: false }),
  MixPlayground: dynamic(() => import("../playground/MixPlayground").then((m) => m.MixPlayground)),
  MessagingDemo: dynamic(() => import("../playground/messaging-section").then((m) => m.MessagingDemo)),
  RewardsCalculator: dynamic(() => import("../operators/interactive/calculators/reward-calculator")),
};

export function Demo({ name, ...props }: { name: string } & Record<string, unknown>) {
  const Component = Object.hasOwn(DEMOS, name) ? DEMOS[name] : null;
  if (!Component) throw new Error(`<Demo name="${name}">: not one of ${Object.keys(DEMOS).join(", ")}`);
  return <Component {...props} />;
}
