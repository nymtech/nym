import React, { useState, useEffect, useRef, useCallback } from 'react';
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
import { useParams } from 'react-router-dom';
import { useMainContext } from '../../context/main';
import { MixnodeStatusWithAll, toMixnodeStatus } from '../../typeDefs/explorer-api';
import { EnumFilterKey, TFilterItem, TFilters } from '../../typeDefs/filters';
import { formatOnSave, generateFilterSchema } from './filterSchema';
import { Api } from '../../api';

const FilterItem = ({
  label,
  id,
  value,
  marks,
  scale,
  min,
  max,
  onChange,
}: TFilterItem & {
  onChange: (id: EnumFilterKey, newValue: number[]) => void;
}) => (
  <Box sx={{ p: 2 }}>
    <Typography gutterBottom>{label}</Typography>
    <Slider
      value={value}
      onChange={(e: Event, newValue: number | number[]) => onChange(id, newValue as number[])}
      valueLabelDisplay="off"
      marks={marks}
      step={null}
      scale={scale}
      min={min}
      max={max}
    />
  </Box>
);

export const Filters = () => {
  const { filterMixnodes, fetchMixnodes } = useMainContext();
  const { status } = useParams<{ status: MixnodeStatusWithAll | undefined }>();

  const [showFilters, setShowFilters] = useState(false);
  const [isFiltered, setIsFiltered] = useState(false);
  const [filters, setFilters] = React.useState<TFilters>();

  const baseFilters = useRef<TFilters>();
  const prevFilters = useRef<TFilters>();

  const handleToggleShowFilters = () => setShowFilters(!showFilters);

  const initialiseFilters = useCallback(async () => {
    let upperSaturationValue;
    const allMixnodes = await Api.fetchMixnodes();
    if (allMixnodes) {
      upperSaturationValue = Math.round(Math.max(...allMixnodes.map((m) => m.stake_saturation)) * 100 + 1);
      const initFilters = generateFilterSchema(upperSaturationValue);
      baseFilters.current = initFilters;
      prevFilters.current = initFilters;
      setFilters(initFilters);
    }
  }, []);

  const handleOnChange = (id: EnumFilterKey, newValue: number[]) => {
    setFilters((ftrs) => {
      if (ftrs) return { ...ftrs, [id]: { ...ftrs[id], value: newValue } };
      return undefined;
    });
  };

  const handleOnSave = async () => {
    handleToggleShowFilters();
    await filterMixnodes(formatOnSave(filters!), status);
    setIsFiltered(true);
    prevFilters.current = filters;
  };

  const handleOnCancel = () => {
    setShowFilters(false);
    setFilters(prevFilters.current);
  };

  const onClearFilters = async () => {
    await fetchMixnodes(toMixnodeStatus(status));
    setFilters(baseFilters.current);
    setIsFiltered(false);
    prevFilters.current = baseFilters.current;
  };

  useEffect(() => {
    initialiseFilters();
  }, [initialiseFilters]);

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
          {Object.values(filters).map((v) => (
            <FilterItem {...v} key={v.id} onChange={handleOnChange} />
          ))}
        </DialogContent>
        <DialogActions>
          <Button size="large" onClick={handleOnCancel}>
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
