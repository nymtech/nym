import { useEffect, useState, useCallback } from "react";
import CircularProgress from "@mui/material/CircularProgress";
import Button from "@mui/material/Button";
import Input from "@mui/material/Input";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";

import { createMixFetch } from "@nymproject/mix-fetch-full-fat";

export const MixFetch = () => {
  const [client, setClient] = useState<any>();
  const [url, setUrl] = useState<string>();
  const [html, setHtml] = useState<any>();

  const connectMixFetch = async () => {
    const mixFetch = await createMixFetch();
    setClient(mixFetch);
  };

  useEffect(() => {
    connectMixFetch();
  }, []);

  if (!client) {
    return (
      <Box sx={{ display: "flex" }}>
        <CircularProgress />
      </Box>
    );;
  }

  return (
    <div style={{ marginTop: "1rem" }}>
      <Box>
        <Typography variant="body1">Enter a url to fetch:</Typography>
        <Input type="text" onChange={(e) => setUrl(e.target.value)} />
        <Button
          variant="contained"
          disabled={!url || !client}
          sx={{ marginLeft: "1rem" }}
          onClick={async () => {
            try {
              const response = await client.mixFetch(url);
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
      </Box>
    </div>
  );
};
