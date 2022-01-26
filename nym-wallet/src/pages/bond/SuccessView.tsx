import React, { useContext } from 'react'
import { Box } from '@mui/system'
import { SuccessReponse, TransactionDetails } from '../../components'
import { ClientContext } from '../../context/main'

export const SuccessView: React.FC<{ details?: { amount: string; address: string } }> = ({ details }) => {
  const { userBalance, currency } = useContext(ClientContext)
  return (
    <>
      <SuccessReponse
        title="Bonding Complete"
        subtitle="Successfully bonded to node with following details"
        caption={`Your current balance is: ${userBalance.balance?.printable_balance}`}
      />
      {details && (
        <Box sx={{ mt: 2 }}>
          <TransactionDetails
            details={[
              { primary: 'Node', secondary: details.address },
              { primary: 'Amount', secondary: `${details.amount} ${currency?.major}` },
            ]}
          />
        </Box>
      )}
    </>
  )
}
