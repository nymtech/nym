import React, { useContext } from 'react'
import { ClientContext } from '../context/main'

export const VestingTimeline: React.FC = () => {
  const { userBalance } = useContext(ClientContext)

  const arr = new Array(userBalance.originalVesting?.number_of_periods).fill(undefined)
  return (
    <svg width="250" height="20" viewBox="0 0 250 8">
      <rect y="2" width="242px" height="8" rx="0" fill="#E6E6E6" />
      <rect y="2" width={25} height="8" rx="0" fill="#121726" />
      <rect width="4" height="12" rx="1" fill="#121726" />
      {arr.map((e, i) => (
        <rect x={(i + 1) * 30} width="4" height="12" rx="1" fill="#B9B9B9" />
      ))}
    </svg>
  )
}
