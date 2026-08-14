# Phase 2 prompt (queued, sent when Phase 1 returns)

Same rules as Phase 1: the MCP server is the only source of truth, no reading the
repo outside `mcp-trial/`, no WebFetch or WebSearch, no answering from priors.

---

Good. Now the actual decision, which is what I need you for.

Two products, and I need to know how to route each one. Work out the answer from
the documentation rather than from instinct, and show me the route you took.

**Product A.** A cryptocurrency wallet. It syncs by talking to a third-party RPC
endpoint we do not run and cannot change. Our concern is that the endpoint
operator builds a profile: linking the addresses we query to each other, and to
us, across sessions.

**Product B.** A messaging feature inside our own app. Both ends run our
software. We control the client on both sides.

Both ship as a desktop Rust application and a browser client.

For each product, tell me:

1. **The threat model first.** Which adversary actually matters here, using the
   documentation's own vocabulary? Which linkage vectors are open, and which
   unlinkability properties are at risk?
2. **The configuration that follows.** End-to-end or proxy mode, and why the
   threat model forces that answer rather than the other one.
3. **What the transport does not solve.** Be specific. If there is residual risk
   the configuration leaves open, name it and say what the documentation says to
   do about it.
4. **The concrete packages**, for both the Rust and browser targets.

Then, across both products:

5. Is there anything the documentation warns you *not* to conclude here? A common
   mistake it explicitly calls out? If so, quote it and say whether your own first
   instinct would have made it.

Finally, and separately from the answer itself:

6. **How did you find this?** List the pages you went through, in order. Did the
   documentation lead you from "what am I defending against" to "which package do
   I install", or did you have to assemble that path yourself from pages that did
   not link to each other? Name any point where you had to guess which page to
   look at next.

Question 6 matters as much as 1 to 5. Be blunt about it.
