import { useEffect, useState, useCallback } from "react";
import CircularProgress from "@mui/material/CircularProgress";
import Button from "@mui/material/Button";
import Input from "@mui/material/Input";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";

import { mixFetch } from "@nymproject/mix-fetch-full-fat";

const args = { mode: 'unsafe-ignore-cors' };

const defaultUrl = 'https://nymtech.net/';

export const MixFetch = () => {
  const [url, setUrl] = useState<string>(defaultUrl);
  const [html, setHtml] = useState<any>();

  return (
    <div style={{ marginTop: "1rem" }}>
      <Box>
        <Typography variant="body1">Enter a url to fetch:</Typography>
        <Input type="text" defaultValue={defaultUrl} onChange={(e) => setUrl(e.target.value)} />
        <Button
          variant="contained"
          disabled={!url}
          sx={{ marginLeft: "1rem" }}
          onClick={async () => {
            try {
              const response = await mixFetch(url, args, {
                preferredGateway: 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM', // with WSS
                preferredNetworkRequester:
                  'GiRjFWrMxt58pEMuusm4yT3RxoMD1MMPrR9M2N4VWRJP.3CNZBPq4vg7v7qozjGjdPMXcvDmkbWPCgbGCjQVw9n6Z@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW',
                  mixFetchOverride: {
                    requestTimeoutMs: 60_000,
                  },
              });
              console.log(response);
              const html = await response.text();
              setHtml(html);
            }
            catch (err) {
              console.log(err);
            }
          }}
        >
          Fetch
        </Button>
        <pre>{html}</pre>
      </Box>
    </div>
  );
};
