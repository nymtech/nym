"use client";

// Client-only wrappers for the interactive demo/diagram components. These were previously
// created with next/dynamic(..., { ssr: false }) inside each MDX page, but an MDX page is a
// server component under the App Router, and passing a dynamic LoadableComponent (a function)
// across the server/client boundary throws "Functions cannot be passed directly to Client
// Components". Declaring the dynamic imports in this "use client" module is legal, keeps the
// ssr:false client-only rendering (the demos load WASM / use browser APIs), and the MDX pages
// just import the ready components from here.
import dynamic from "next/dynamic";

export const NetworkDiagram = dynamic(
  () => import("./threat-model/NetworkDiagram").then((m) => m.NetworkDiagram),
  { ssr: false },
);
export const DvpnCoverTraffic = dynamic(
  () => import("./threat-model/DvpnCoverTraffic").then((m) => m.DvpnCoverTraffic),
  { ssr: false },
);
export const MixnetDeepDive = dynamic(
  () => import("./threat-model/MixnetDeepDive").then((m) => m.MixnetDeepDive),
  { ssr: false },
);
export const PacketAnatomy = dynamic(
  () => import("./threat-model/PacketAnatomy").then((m) => m.PacketAnatomy),
  { ssr: false },
);
export const SurbDirectory = dynamic(
  () => import("./threat-model/SurbDirectory").then((m) => m.SurbDirectory),
  { ssr: false },
);
export const RailgunDemo = dynamic(
  () => import("./demos/railgun/RailgunDemo").then((m) => m.RailgunDemo),
  { ssr: false },
);
export const EnsDemo = dynamic(
  () => import("./demos/ens/EnsDemo").then((m) => m.EnsDemo),
  { ssr: false },
);
