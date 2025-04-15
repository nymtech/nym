"use client";
import type { IObservatoryNode } from "@/app/api/types";
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
import { fetchObservatoryNodes } from "../../app/api";

const NodeAndAddressSearch = () => {
  const router = useRouter();
  const [inputValue, setInputValue] = useState("");
  const [errorText, setErrorText] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [searchOptions, setSearchOptions] = useState<IObservatoryNode[]>([]);

  // Use React Query to fetch nodes
  const { data: nymNodes = [], isLoading: isLoadingNodes } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const handleSearch = async () => {
    setErrorText(""); // Clear any previous error messages
    setIsLoading(true); // Start loading

    try {
      if (inputValue.startsWith("n1")) {
        // Fetch Nym Address data
        const response = await fetch(`${NYM_ACCOUNT_ADDRESS}/${inputValue}`);

        if (response.ok) {
          try {
            const data = await response.json();
            if (data) {
              router.push(`/account/${inputValue}`);
              return;
            }
          } catch {
            setErrorText(
              "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again.",
            );
            setIsLoading(false); // Stop loading

            return;
          }
        } else {
          setErrorText(
            "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again.",
          );
          setIsLoading(false); // Stop loading

          return;
        }
      } else {
        // Check if it's a node identity key
        if (nymNodes) {
          const matchingNode = nymNodes.find(
            (node) => node.identity_key === inputValue,
          );

          if (matchingNode) {
            router.push(`/nym-node/${matchingNode.identity_key}`);
            return;
          }
        }
        setErrorText(
          "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again.",
        );
        setIsLoading(false);
      }
    } catch (error) {
      setErrorText(
        "No node found with the provided Name, Node ID or Identity Key. Please check your input and try again.",
      );
      console.error(error);
      setIsLoading(false); // Stop loading
    }
  };

  // Handle search input change
  const handleSearchInputChange = (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value = event.target.value;
    setInputValue(value);

    // Clear error message when input is empty
    if (!value.trim()) {
      setErrorText("");
    }

    // Filter nodes by moniker if input is not empty
    if (value.trim() !== "") {
      const filteredNodes = nymNodes.filter((node) =>
        node.self_description?.moniker
          ?.toLowerCase()
          .includes(value.toLowerCase()),
      );
      setSearchOptions(filteredNodes);
    } else {
      setSearchOptions([]);
    }
  };

  // Handle node selection from dropdown
  const handleNodeSelect = (
    event: React.SyntheticEvent,
    value: string | IObservatoryNode | null,
  ) => {
    if (value && typeof value !== "string") {
      setIsLoading(true); // Show loading spinner
      router.push(`/nym-node/${value.node_id}`);
    }
  };

  return (
    <Stack spacing={1} direction="column">
      <Stack spacing={4} direction="row">
        <Autocomplete
          freeSolo
          options={searchOptions}
          getOptionLabel={(option: string | IObservatoryNode) => {
            if (typeof option === "string") return option;
            return option.self_description?.moniker || "";
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
                key={`${option.node_id}-${option.self_description?.moniker || ""}`}
                style={{ fontSize: "0.875rem" }}
              >
                {option.self_description?.moniker || "Unnamed Node"}
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
          loading={isLoadingNodes}
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
