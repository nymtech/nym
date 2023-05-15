import React from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowBackIosRounded } from '@mui/icons-material';
import { IconButton } from '@mui/material';

export const BackButton = () => {
  const navigate = useNavigate();
  return (
    <IconButton size="small" onClick={() => navigate(-1)}>
      <ArrowBackIosRounded fontSize="small" />
    </IconButton>
  );
};
