import React from 'react';
import { Button, IconButton } from '@mui/material';
import { Tune } from '@mui/icons-material';

type FiltersButtonProps = {
  iconOnly?: boolean;
  fullWidth?: boolean;
  onClick: () => void;
};

const FiltersButton = ({ iconOnly, fullWidth, onClick }: FiltersButtonProps) => {
  if (iconOnly) {
    return (
      <IconButton onClick={onClick} color="primary">
        <Tune />
      </IconButton>
    );
  }

  return (
    <Button
      fullWidth={fullWidth}
      size="large"
      variant="contained"
      endIcon={<Tune />}
      onClick={onClick}
      sx={{ textTransform: 'none' }}
    >
      Filters
    </Button>
  );
};

export default FiltersButton;
