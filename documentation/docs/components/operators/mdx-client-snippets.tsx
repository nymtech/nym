"use client";

// Small presentational snippets that MDX pages pass to the client AccordionTemplate (as its
// `name`) or render alongside other client components. Defined inline in MDX they are server
// component functions, and passing one across the client boundary throws "Functions cannot be
// passed directly to Client Components". Declaring them in this "use client" module makes each
// <X/> a serializable client reference; the pages import them and use <X/> unchanged.

export const NymNodeCliCommand = () => (
  <div>
    Arguments and options: <code>./nym-node-cli.py install --help</code>
  </div>
);

export const TestingSteps = () => <div>Testing steps performed</div>;

export const TryYourself = () => <div>Try yourself</div>;

export const CiConfig = () => (
  <div>
    Components of <code>ci-binary-config-checker</code>
  </div>
);

export const TunnelManagerCommands = () => (
  <div>
    Commands to update IP tables rules with a new <code>network_tunnel_manager.sh</code>
  </div>
);

export const LoadEndpointInfo = () => (
  <div>
    Developer notes behind <code>/load</code> endpoint
  </div>
);

export const IndexPage = () => (
  <>
    An example template for <code>index.html</code> page
  </>
);
