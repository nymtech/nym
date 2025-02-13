"use client";
// Types

import Arrow from "../../../public/icons/arrow.svg";

// MUI Components
import { Box, Button, Input } from "@mui/material";
import Image from "next/image";

export const NewsletterSignUp = () => {
  return (
    <Box
      sx={{
        display: "flex",
        gap: "10px",
        justifyContent: "space-between",
        alignItems: "center",
        border: "1px solid",
        borderColor: "background.main",
        padding: "3px 4px 3px 20px",
        borderRadius: "54px",
        boxShadow: "2px 2px 4px 0px rgba(0, 0, 0, 0.25) inset",
        "&:focus-within": {
          border: "1px solid #000",
        },
        ":has(input:error)": {
          border: "1px solid red",
        },
        backgroundColor: "base.white",
        flexGrow: 1,
        maxWidth: "400px",
      }}
    >
      <Input
        placeholder="Enter your email address"
        type="email"
        required
        disableUnderline={true}
        sx={{
          width: "100%",
          position: "relative",
          top: "1px",
        }}
      />
      <Button
        sx={{
          width: "47px",
          height: "47px",
          backgroundColor: "accent.main",
          borderRadius: "50%",
          minWidth: "47px",
          color: "background.main",
          padding: "15px",
          "&:hover": {
            backgroundColor: "accent.main",
            color: "background.main",
          },
        }}
      >
        <Image src={Arrow} alt={"arrow icon"} width={15} height={15} />
      </Button>
    </Box>
  );
};
