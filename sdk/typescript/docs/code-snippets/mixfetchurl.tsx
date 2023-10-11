import React, { useState } from 'react';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Stack from '@mui/material/Stack';
import Box from '@mui/material/Box';

export const GitHubRepoSearch = () => {
  const [repoUrl, setRepoUrl] = useState('');
  
  const handleSearch = () => {
    if(!repoUrl || repoUrl.length < 1 ) {
        return window.alert("Please enter a valid Github URL!")
    } 
    const matchedRepo = repoUrl.match(/https:\/\/github\.com\/(.*)/)[1]

    // Construct the search URL
    const searchUrl = `https://github.com/search?q=repo:${matchedRepo} fetch(&type=code`;

    // Redirect the user to a new search results page
    window.open(searchUrl, "_blank");
  };

  return (
    <Box padding={3}>
      <Box>
        <TextField
          type="text"
          placeholder="Enter GitHub repo URL: https://github.com/nymtech/nym/"
          value={repoUrl}
          onChange={(e) => setRepoUrl(e.target.value)}
          size="small"
          sx={{width: "450px"}}
        />
  
        <Button
          variant="outlined"
          onClick={handleSearch}
          size="medium"
          sx={{ marginLeft: 2, marginTop: 0.2 }}
        >
          Check mixFetch
        </Button>
      </Box>
    </Box>
  );
}

