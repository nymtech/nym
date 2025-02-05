"use client";
import type { NodeData } from "@/app/api/types";
import { Search } from "@mui/icons-material";
import { Button, CircularProgress, Stack, Typography } from "@mui/material";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { NYM_NODE_BONDED } from "../../app/api/urls";
import Input from "../input/Input";

const NodeAndAddressSearch = () => {
  const router = useRouter();
  const [inputValue, setInputValue] = useState("");
  const [errorText, setErrorText] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const handleSearch = async () => {
    setErrorText(""); // Clear any previous error messages
    setIsLoading(true); // Start loading

    try {
      if (inputValue.startsWith("n1")) {
        // Fetch Nym Address data
        const response = await fetch(
          `https://explorer.nymtech.net/api/v1/tmp/unstable/account/${inputValue}`,
        );

        if (response.ok) {
          try {
            const data = await response.json();
            if (data) {
              router.push(`/account/${inputValue}`);
              return;
            }
          } catch {
            setErrorText(
              "It seems that this node or account does not exist. Please enter a complete Node ID or an existing Nym wallet address.",
            );
            return;
          }
        } else {
          setErrorText(
            "It seems that this node or account does not exist. Please enter a complete Node ID or an existing Nym wallet address.",
          );
          return;
        }
      } else {
        // Fetch Nym Nodes data
        const response = await fetch(NYM_NODE_BONDED);

        if (response.ok) {
          const nodes = await response.json();
          const matchingNode = nodes.data.find(
            (node: NodeData) =>
              node.bond_information.node.identity_key === inputValue,
          );

          if (matchingNode) {
            router.push(`/nym-node/${matchingNode.bond_information.node_id}`);
            return;
          }
        }
        setErrorText(
          "It seems that this node or account does not exist. Please enter a complete Node ID or an existing Nym wallet address.",
        );
      }
    } catch (error) {
      setErrorText(
        "It seems that this node or account does not exist. Please enter a complete Node ID or an existing Nym wallet address.",
      );
    } finally {
      setIsLoading(false); // Stop loading
    }
  };

  return (
    <Stack spacing={2} direction="column">
      <Stack spacing={4} direction="row">
        <Input
          placeholder="Node Identity Key / Nym Address"
          fullWidth
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleSearch();
            }
          }}
          rounded
        />
        <Button
          variant="contained"
          endIcon={
            isLoading ? (
              <CircularProgress size={24} color="inherit" />
            ) : (
              <Search />
            )
          }
          size="large"
          onClick={handleSearch}
          sx={{ height: "56px" }} // Match the TextField height
        >
          Search
        </Button>
      </Stack>
      {errorText && (
        <Typography color="error" variant="body4">
          {errorText}
        </Typography>
      )}
    </Stack>
  );
};

export default NodeAndAddressSearch;
