"use client";

import { Box } from "@mui/material";
import Script from "next/script";
import { useEffect } from "react";

declare global {
  interface Window {
    remark_config: {
      host: string;
      site_id: string;
      components: string[];
      max_shown_comments: number;
      theme: string;
      locale: string;
      show_email_subscription: boolean;
      simple_view: boolean;
      no_footer: boolean;
    };
    REMARK42: {
      createInstance: (config: typeof window.remark_config) => void;
      changeTheme: (theme: "light" | "dark") => void;
    };
  }
}

export const Remark42Comments = () => {
  useEffect(() => {
    if (typeof window !== "undefined") {
      // Set Remark42 configuration on the window object
      window.remark_config = {
        host: "https://remark42.nymte.ch",
        site_id: "remark",
        components: ["embed", "last-comments"],
        max_shown_comments: 100,
        theme: "light",
        locale: "en",
        show_email_subscription: false,
        simple_view: true,
        no_footer: true,
      };

      // Dynamically load the Remark42 script if it doesn't exist
      if (!document.getElementById("remark42-script")) {
        const script = document.createElement("script");
        script.src = `${window.remark_config.host}/web/embed.js`;
        script.async = true;
        script.defer = true;
        script.id = "remark42-script";
        document.body.appendChild(script);
      } else if (window.REMARK42) {
        // Re-initialize if the script is already loaded
        window.REMARK42.createInstance(window.remark_config);
      }
    }
  }, []);

  // React to mode changes and update Remark42 theme
  //   useEffect(() => {
  //     if (window.REMARK42 && window.REMARK42.changeTheme) {
  //       window.REMARK42.changeTheme(mode === "dark" ? "dark" : "light");
  //     }
  //   }, [mode]);

  return (
    <Box>
      <div id="remark42" className="remark" />
      <Script
        id="remark-init"
        strategy="afterInteractive"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: <explanation>
        dangerouslySetInnerHTML={{
          __html: `
          if (window.REMARK42) {
            window.REMARK42.createInstance(window.remark_config);
          }
        `,
        }}
      />
    </Box>
  );
};
