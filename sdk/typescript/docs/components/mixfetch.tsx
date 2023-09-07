import { useEffect, useState, useCallback } from "react";
import CircularProgress from "@mui/material/CircularProgress";
import Button from "@mui/material/Button";
import Input from "@mui/material/Input";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";

import { createMixFetch } from "@nymproject/mix-fetch";

export const MixFetch = () => {
  const [mixFetch, setMixFetch] = useState<any>();
  const [url, setUrl] = useState<string>();
  const [html, setHtml] = useState<any>();

  const connectMixFetch = async () => {
    const mixFetch = await createMixFetch();
    setMixFetch(mixFetch);
    console.log(mixFetch);
  };

  useEffect(() => {
    connectMixFetch();
  }, []);

  if (!mixFetch) {
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
          disabled={!url || !mixFetch}
          sx={{ marginLeft: "1rem" }}
          onClick={async () => {
            const response = await mixFetch(url);
            const html = await response.text();
            setHtml(html);
          }}
        >
          Fetch
        </Button>
      </Box>
    </div>
  );
};
