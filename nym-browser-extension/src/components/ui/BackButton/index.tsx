import React from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowBackIosRounded } from '@mui/icons-material';
import { IconButton } from '@mui/material';

export const BackButton = ({ onBack }: { onBack?: () => void }) => {
  const navigate = useNavigate();

  const handleClick = () => {
    if (onBack) {
      onBack();
    } else {
      navigate(-1);
    }
    return undefined;
  };

  return (
    <IconButton size="small" onClick={handleClick}>
      <ArrowBackIosRounded fontSize="small" />
    </IconButton>
  );
};
