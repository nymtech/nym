import React from "react";
import { Navbar } from "./components/Nav/Navbar";
import { Providers } from "./providers";

const App = ({ children }: { children: any }) => (
  <Providers>
    <Navbar>{children}</Navbar>
  </Providers>
);

export { App };
