import React, { useState, useEffect, useMemo, useRef } from 'react';
import { Tune } from '@mui/icons-material';
import {
  Button,
  Dialog,
  DialogContent,
  DialogActions,
  DialogTitle,
  IconButton,
  Slider,
  Typography,
  Box,
  Snackbar,
  Slide,
  Alert,
} from '@mui/material';
import { Mark } from '@mui/base';
import { useParams } from 'react-router-dom';
import { useMainContext } from '../../context/main';
import { MixNodeResponseItem, MixnodeStatusWithAll, toMixnodeStatus } from '../../typeDefs/explorer-api';

enum EnumFilterKey {
  profitMargin = 'profitMargin',
  stakeSaturation = 'stakeSaturation',
}

type TFilterItem = {
  label: string;
  id: EnumFilterKey;
  value: number[];
  marks: Mark[];
};

export type TFilters = { [key in EnumFilterKey]: TFilterItem };

const initializeFilters = (mixnodes?: MixNodeResponseItem[]): TFilters => {
  const upperSaturationValue = mixnodes && Math.round(Math.max(...mixnodes.map((m) => m.stake_saturation)) * 100 + 1);

  return {
    profitMargin: {
      label: 'Profit margin (%)',
      id: EnumFilterKey.profitMargin,
      value: [0, 100],
      marks: [
        { label: '0', value: 0 },
        { label: '100', value: 100 },
      ],
    },
    stakeSaturation: {
      label: 'Stake saturation (%)',
      id: EnumFilterKey.stakeSaturation,
      value: [0, upperSaturationValue || 100],
      marks: [
        { label: '0', value: 0 },
        {
          label: upperSaturationValue ? `${upperSaturationValue}` : '100',
          value: upperSaturationValue || 100,
        },
      ],
    },
  };
};

const FilterItem = ({
  label,
  id,
  value,
  marks,
  onChange,
}: TFilterItem & {
  onChange: (id: EnumFilterKey, newValue: number[]) => void;
}) => (
  <Box sx={{ p: 2 }}>
    <Typography gutterBottom>{label}</Typography>
    <Slider
      value={value}
      onChange={(e: Event, newValue: number | number[]) => onChange(id, newValue as number[])}
      max={marks[1].value}
      valueLabelDisplay="auto"
      marks={marks}
    />
  </Box>
);

export const Filters = () => {
  const { filterMixnodes, fetchMixnodes, mixnodes } = useMainContext();
  const { status } = useParams<{ status: MixnodeStatusWithAll | undefined }>();

  const [showFilters, setShowFilters] = useState(false);
  const [isFiltered, setIsFiltered] = useState(false);
  const [filters, setFilters] = React.useState<TFilters>();

  const initialFilers = useRef<TFilters>();

  const handleToggleShowFilters = () => setShowFilters(!showFilters);

  const handleChange = (id: EnumFilterKey, newValue: number[]) => {
    setFilters((currentFilters) => {
      if (currentFilters) return { ...currentFilters, [id]: { ...currentFilters[id], value: newValue } };
      return undefined;
    });
  };

  const handleOnSave = () => {
    filterMixnodes(filters, status);
    handleToggleShowFilters();
    setIsFiltered(true);
  };

  const onClearFilters = async () => {
    await fetchMixnodes(toMixnodeStatus(status));
    setIsFiltered(false);
    setFilters(initialFilers.current);
  };

  useEffect(() => {
    if (!filters && mixnodes?.data) {
      const init = initializeFilters(mixnodes.data);
      initialFilers.current = init;
      setFilters(init);
    }
  }, [mixnodes]);

  if (!filters) return null;

  return (
    <>
      <Snackbar
        open={isFiltered}
        anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
        message="Filters applied"
        TransitionComponent={Slide}
        transitionDuration={250}
      >
        <Alert
          severity="info"
          variant="outlined"
          action={
            <Button size="small" onClick={onClearFilters}>
              Clear
            </Button>
          }
          sx={{ width: 300 }}
        >
          Filters applied
        </Alert>
      </Snackbar>
      <IconButton size="large" onClick={handleToggleShowFilters}>
        <Tune />
      </IconButton>
      <Dialog open={showFilters} onClose={handleToggleShowFilters} maxWidth="md" fullWidth>
        <DialogTitle>Mixnode filters</DialogTitle>
        <DialogContent dividers>
          <FilterItem
            label={filters.profitMargin.label}
            id={EnumFilterKey.profitMargin}
            value={filters.profitMargin.value}
            onChange={handleChange}
            marks={filters.profitMargin.marks}
          />
          <FilterItem
            label={filters.stakeSaturation.label}
            id={EnumFilterKey.stakeSaturation}
            value={filters.stakeSaturation.value}
            onChange={handleChange}
            marks={filters.stakeSaturation.marks}
          />
        </DialogContent>
        <DialogActions>
          <Button size="large" onClick={handleToggleShowFilters}>
            Cancel
          </Button>
          <Button variant="contained" size="large" onClick={handleOnSave}>
            Save
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};
