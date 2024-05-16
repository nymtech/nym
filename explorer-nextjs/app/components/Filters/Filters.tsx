'use client'

import React, { useState, useEffect, useRef, useCallback } from 'react'
import {
  Button,
  Dialog,
  DialogContent,
  DialogActions,
  DialogTitle,
  Slider,
  Typography,
  Box,
  Snackbar,
  Slide,
  Alert,
} from '@mui/material'
import { useParams } from 'next/navigation'
import { useMainContext } from '@/app/context/main'
import {
  MixnodeStatusWithAll,
  toMixnodeStatus,
} from '@/app/typeDefs/explorer-api'
import { EnumFilterKey, TFilterItem, TFilters } from '@/app/typeDefs/filters'
import { Api } from '@/app/api'
import { useIsMobile } from '@/app/hooks/useIsMobile'
import { formatOnSave, generateFilterSchema } from './filterSchema'
import FiltersButton from './FiltersButton'

const FilterItem = ({
  label,
  id,
  tooltipInfo,
  value,
  isSmooth,
  marks,
  scale,
  min,
  max,
  onChange,
}: TFilterItem & {
  onChange: (id: EnumFilterKey, newValue: number[]) => void
}) => (
  <Box sx={{ p: 2 }}>
    <Typography gutterBottom>{label}</Typography>
    <Typography fontSize={12}>{tooltipInfo}</Typography>
    <Slider
      value={value}
      onChange={(e: Event, newValue: number | number[]) =>
        onChange(id, newValue as number[])
      }
      valueLabelDisplay={isSmooth ? 'auto' : 'off'}
      marks={marks}
      step={isSmooth ? 1 : null}
      scale={scale}
      min={min}
      max={max}
      valueLabelFormat={(val: number) =>
        val === 100 && id === 'stakeSaturation' ? '>100' : val
      }
    />
  </Box>
)

export const Filters = () => {
  const { filterMixnodes, fetchMixnodes, mixnodes } = useMainContext()
  const { status } = useParams<{
    status: 'active' | 'standby' | 'inactive' | 'all'
  }>()
  const isMobile = useIsMobile()

  const [showFilters, setShowFilters] = useState(false)
  const [isFiltered, setIsFiltered] = useState(false)
  const [filters, setFilters] = React.useState<TFilters>()
  const [upperSaturationValue, setUpperSaturationValue] =
    React.useState<number>(100)

  const baseFilters = useRef<TFilters>()
  const prevFilters = useRef<TFilters>()

  const handleToggleShowFilters = () => setShowFilters(!showFilters)

  const initialiseFilters = useCallback(async () => {
    const allMixnodes = await Api.fetchMixnodes()
    if (allMixnodes) {
      setUpperSaturationValue(
        Math.round(
          Math.max(...allMixnodes.map((m) => m.stake_saturation)) * 100 + 1
        )
      )
      const initFilters = generateFilterSchema()
      baseFilters.current = initFilters
      prevFilters.current = initFilters
      setFilters(initFilters)
    }
  }, [])

  const handleOnChange = (id: EnumFilterKey, newValue: number[]) => {
    if (id === 'stakeSaturation' && newValue[1] === 100) {
      newValue.splice(1, 1, upperSaturationValue)
    }
    setFilters((ftrs) => {
      if (ftrs)
        return {
          ...ftrs,
          [id]: {
            ...ftrs[id],
            value: newValue,
          },
        }
      return undefined
    })
  }

  const handleOnSave = async () => {
    setShowFilters(false)
    await filterMixnodes(formatOnSave(filters!), status)
    setIsFiltered(true)
    prevFilters.current = filters
  }

  const handleOnCancel = () => {
    setShowFilters(false)
    setFilters(prevFilters.current)
  }

  const resetFilters = () => {
    setFilters(baseFilters.current)
    setIsFiltered(false)
    prevFilters.current = baseFilters.current
  }

  const onClearFilters = async () => {
    await fetchMixnodes(toMixnodeStatus(MixnodeStatusWithAll[status]))
    resetFilters()
  }

  useEffect(() => {
    initialiseFilters()
  }, [initialiseFilters])

  useEffect(() => {
    resetFilters()
  }, [status])

  if (!filters) return null

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
          variant={isMobile ? 'standard' : 'outlined'}
          sx={{ color: (t) => t.palette.info.light }}
          action={
            <Button size="small" onClick={onClearFilters}>
              CLEAR FILTERS
            </Button>
          }
        >
          {mixnodes?.data?.length} mixnodes matched your criteria
        </Alert>
      </Snackbar>
      <FiltersButton onClick={handleToggleShowFilters} fullWidth />
      <Dialog
        open={showFilters}
        onClose={handleToggleShowFilters}
        maxWidth="md"
        fullWidth
      >
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
  )
}
