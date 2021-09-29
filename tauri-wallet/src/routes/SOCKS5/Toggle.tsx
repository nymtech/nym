import React, { useState } from 'react'
import { Grid, Paper, Theme, Typography } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'

enum EnumOptions {
  Mb = 'Mb',
  Gb = 'Gb',
}

const ToggleOption = ({
  title,
  isSelected,
  setSelected,
}: {
  title: EnumOptions
  isSelected: boolean
  setSelected: (selection: EnumOptions) => void
}) => {
  const theme: Theme = useTheme()

  return (
    <Typography
      variant="caption"
      onClick={() => setSelected(title)}
      style={{
        cursor: 'pointer',
        color: isSelected ? theme.palette.grey[900] : theme.palette.grey[500],
      }}
    >
      {title}
    </Typography>
  )
}

export const ToggleData = () => {
  const theme: Theme = useTheme()
  const [selected, setSeleted] = useState(EnumOptions['Mb'])
  return (
    <Paper
      elevation={0}
      style={{
        width: 75,
        backgroundColor: theme.palette.grey[100],
      }}
    >
      <Grid container spacing={1} justifyContent="center">
        <Grid item>
          <ToggleOption
            title={EnumOptions['Mb']}
            isSelected={selected === EnumOptions['Mb']}
            setSelected={(selection) => setSeleted(selection)}
          />
        </Grid>
        <Grid item>
          <ToggleOption
            title={EnumOptions['Gb']}
            isSelected={selected === EnumOptions['Gb']}
            setSelected={(selection) => setSeleted(selection)}
          />
        </Grid>
      </Grid>
    </Paper>
  )
}
