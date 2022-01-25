import React, { useState, useEffect } from 'react'
import { Typography } from '@mui/material'
import { MAJOR_CURRENCY } from '../context/main'
import { Operation } from '../types'
import { getGasFee } from '../requests'

export const Fee = ({ feeType }: { feeType: Operation }) => {
  const [fee, setFee] = useState<string>()

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
        Fee for this transaction: {`${fee} ${MAJOR_CURRENCY}`}{' '}
      </Typography>
    )
  }

  return null
}
