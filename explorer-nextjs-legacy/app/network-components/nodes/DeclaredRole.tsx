import React from "react";
import { Chip } from "@mui/material";

export const DeclaredRole = ({ declared_role }: { declared_role?: any }) => (
  <>
    {declared_role?.mixnode && <Chip size="small" label="Mixnode" sx={{ mr: 0.5 }} color="info" />}
    {declared_role?.entry && <Chip size="small" label="Entry" sx={{ mr: 0.5 }} color="success" />}
    {declared_role?.exit_nr && <Chip size="small" label="Exit NR" sx={{ mr: 0.5 }} color="warning" />}
    {declared_role?.exit_ipr && <Chip size="small" label="Exit IPR" sx={{ mr: 0.5 }} color="warning" />}
  </>
)