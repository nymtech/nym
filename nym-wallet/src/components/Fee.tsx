import React, { useState, useEffect, useContext } from 'react'
import { Typography } from '@mui/material'
import { Operation } from '../types'
import { getGasFee } from '../requests'
import { ClientContext } from '../context/main'

export const Fee = ({ feeType }: { feeType: Operation }) => {
  const [fee, setFee] = useState<string>()
  const {currency} = useContext(ClientContext)

  const getFee = async () => {
    const fee = await getGasFee(feeType)
    setFee(fee.amount)
  }

  useEffect(() => {
    getFee()
  }, [])

  if (fee) {
    return (
      <Typography sx={{ color: 'nym.fee', fontWeight: 600 }}>
        Fee for this transaction: {`${fee} ${currency?.major}`}{' '}
      </Typography>
    )
  }

  return null
}
