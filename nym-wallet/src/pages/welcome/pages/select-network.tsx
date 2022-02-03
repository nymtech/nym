import React, { useContext } from 'react'
import { FormControl, FormControlLabel, Radio, RadioGroup, Stack } from '@mui/material'
import { Network } from '../../../types'
import { ClientContext } from '../../../context/main'

export const SelectNetwork: React.FC<{ page: 'select network' }> = () => {
  const { network, switchNetwork } = useContext(ClientContext)

  return (
    <Stack alignItems="center" spacing={5}>
      <FormControl>
        <RadioGroup
          aria-labelledby="demo-controlled-radio-buttons-group"
          name="controlled-radio-buttons-group"
          value={network}
          onChange={(e) => switchNetwork(e.target.value as Network)}
          row
        >
          <FormControlLabel value="SANDBOX" control={<Radio color="default" />} label="Testnet Sandbox" />
          <FormControlLabel value="QA" control={<Radio color="default" />} label="QA" />
        </RadioGroup>
      </FormControl>
    </Stack>
  )
}
