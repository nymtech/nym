"use client";
import type { NS_NODE } from "@/app/api/types";
import { NYM_ACCOUNT_ADDRESS } from "@/app/api/urls";
import { Search } from "@mui/icons-material";
import {
  Autocomplete,
  Button,
  CircularProgress,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { fetchNSApiNodes } from "../../app/api";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { getBasePathByEnv } from "../../../envs/config";

const NodeAndAddressSearch = () => {
  const router = useRouter();
  const { environment } = useEnvironment();
  const [inputValue, setInputValue] = useState("");
  const [errorText, setErrorText] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [searchOptions, setSearchOptions] = useState<NS_NODE[]>([]);

  // Use React Query to fetch nodes
  const { data: nsApiNodes = [], isLoading: isNSApiNodesLoading } = useQuery({
    queryKey: ["nsApiNodes", environment],
    queryFn: () => fetchNSApiNodes(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const handleSearch = async () => {
    if (!inputValue.trim()) {
      setErrorText("Please enter a search term");
      return;
    }

    setIsLoading(true);
    setErrorText("");

    try {
      if (inputValue.startsWith("n1")) {
        // Fetch Nym Address data
        const response = await fetch(`${NYM_ACCOUNT_ADDRESS}/${inputValue}`);

        if (response.ok) {
          try {
            const data = await response.json();
            if (data) {
              const basePath = getBasePathByEnv(environment || "mainnet");
              router.push(`${basePath}/account/${inputValue}`);
              return;
            }
          } catch {
            setErrorText(
              "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again."
            );
            setIsLoading(false); // Stop loading

            return;
          }
        } else {
          setErrorText(
            "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again."
          );
          setIsLoading(false); // Stop loading

          return;
        }
      } else {
        // Check if it's a node identity key
        if (nsApiNodes) {
          const matchingNode = nsApiNodes.find(
            (node: NS_NODE) => node.identity_key === inputValue
          );

          if (matchingNode) {
            const basePath = getBasePathByEnv(environment || "mainnet");
            router.push(`${basePath}/nym-node/${matchingNode.identity_key}`);
            return;
          }
        }
        setErrorText(
          "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again."
        );
        setIsLoading(false);
      }
    } catch (error) {
      setErrorText(
        "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again."
      );
      console.error(error);
      setIsLoading(false); // Stop loading
    }
  };

  // Handle search input change
  const handleSearchInputChange = (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const value = event.target.value;
    setInputValue(value);

    // Clear error message when input is empty
    if (!value.trim()) {
      setErrorText("");
    }

    // Filter nodes by moniker if input is not empty
    if (value.trim() !== "") {
      const filteredNodes = nsApiNodes.filter((node: NS_NODE) =>
        node.description.moniker?.toLowerCase().includes(value.toLowerCase())
      );
      setSearchOptions(filteredNodes);
    } else {
      setSearchOptions([]);
    }
  };

  // Handle node selection from dropdown
  const handleNodeSelect = (
    event: React.SyntheticEvent,
    value: string | NS_NODE | null
  ) => {
    if (value && typeof value !== "string") {
      setIsLoading(true); // Show loading spinner
      const basePath = getBasePathByEnv(environment || "mainnet");
      router.push(`${basePath}/nym-node/${value.node_id}`);
    }
  };

  return (
    <Stack spacing={1} direction="column">
      <Stack spacing={4} direction="row">
        <Autocomplete
          freeSolo
          options={searchOptions}
          getOptionLabel={(option: string | NS_NODE) => {
            if (typeof option === "string") return option;
            return option.description.moniker || "";
          }}
          isOptionEqualToValue={(option, value) => {
            if (typeof option === "string" || typeof value === "string")
              return false;
            return option.node_id === value.node_id;
          }}
          renderOption={(props, option) => {
            if (typeof option === "string") return null;
            return (
              <li
                {...props}
                key={`${option.node_id}-${option.description.moniker || ""}`}
                style={{ fontSize: "0.875rem" }}
              >
                {option.description.moniker || "Unnamed Node"}
              </li>
            );
          }}
          renderInput={(params) => (
            <TextField
              {...params}
              placeholder="Search by Node Name, Identity Key, or Nym Address"
              fullWidth
              onChange={handleSearchInputChange}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  handleSearch();
                }
              }}
              sx={{
                "& .MuiOutlinedInput-root": {
                  borderRadius: "28px",
                  backgroundColor: "background.paper",
                  "& fieldset": {
                    borderColor: "divider",
                  },
                  "&:hover fieldset": {
                    borderColor: "primary.main",
                  },
                  "&.Mui-focused fieldset": {
                    borderColor: "primary.main",
                  },
                },
              }}
            />
          )}
          onChange={handleNodeSelect}
          loading={isNSApiNodesLoading}
          loadingText="Loading nodes..."
          noOptionsText="No nodes found"
          slotProps={{
            paper: {
              sx: {
                marginTop: "4px",
                marginLeft: "10px",
                marginRight: "10px",
                width: "calc(100% - 20px)",
                borderRadius: "10px",
                "& .MuiAutocomplete-listbox": {
                  padding: "8px 0",
                },
              },
            },
          }}
          sx={{
            flexGrow: 1,
            "& .MuiAutocomplete-popupIndicator": {
              color: "text.secondary",
            },
            "& .MuiAutocomplete-clearIndicator": {
              color: "text.secondary",
            },
          }}
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
          onClick={handleSearch}
          disabled={isLoading}
          sx={{
            height: "56px",
            borderRadius: "28px",
          }}
        >
          Search
        </Button>
      </Stack>
      {errorText && (
        <Typography
          variant="caption"
          color="error"
          sx={{
            fontSize: "0.875rem",
            px: 2,
            display: "block",
          }}
        >
          {errorText}
        </Typography>
      )}
    </Stack>
  );
};

export default NodeAndAddressSearch;
