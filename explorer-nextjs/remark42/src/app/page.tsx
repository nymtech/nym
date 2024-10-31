import Head from "next/head";
import Script from "next/script";
import { Box } from "@mui/material";

export default function Home() {
  return (
    <div>
      <Head>
        <title>Create Next App</title>
        <link rel="icon" href="/favicon.ico" />
      </Head>
      <Box
        display={"flex"}
        flexDirection={"column"}
        maxWidth={"60%"}
        margin={"50px auto"}
      >
        <div>NymNode X</div>
        <div>Info about NymNode X</div>
        <div id="remark42"></div>

        {/* Configuration Script */}
        <Script
          id="remark-config"
          strategy="afterInteractive"
          dangerouslySetInnerHTML={{
            __html: `
      var remark_config = {
        host: 'http://localhost:8081', // Updated to match the REMARK_URL
        site_id: 'remark42',
        components: ['embed', 'last-comments'],
        max_shown_comments: 100,
        theme: 'light',
        page_title: 'My custom title for a page',
        locale: 'en',
        show_email_subscription: false,
        simple_view: true,
        no_footer: false
      };
    `,
          }}
        />

        {/* Initialization Script */}
        <Script
          id="remark-init"
          strategy="afterInteractive"
          dangerouslySetInnerHTML={{
            __html: `
            !function(e,n){for(var o=0;o<e.length;o++){var r=n.createElement("script"),c=".js",d=n.head||n.body;"noModule"in r?(r.type="module",c=".mjs"):r.async=!0,r.defer=!0,r.src=remark_config.host+"/web/"+e[o]+c,d.appendChild(r)}}(remark_config.components||["embed"],document);
          `,
          }}
        />
      </Box>
    </div>
  );
}
