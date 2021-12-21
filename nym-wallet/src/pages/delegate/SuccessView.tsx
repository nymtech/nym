import React, { useContext } from 'react'
import { Box } from '@mui/system'
import { SuccessReponse, TransactionDetails } from '../../components'
import { ClientContext, MAJOR_CURRENCY } from '../../context/main'

export const SuccessView: React.FC<{ details?: { amount: string; address: string } }> = ({ details }) => {
  const { userBalance } = useContext(ClientContext)
  return (
    <>
      <SuccessReponse
        title="Delegation Complete"
        subtitle="Successfully delegated to node with following details"
        caption={`Your current balance is: ${userBalance.balance?.printable_balance}`}
      />
      {details && (
        <Box sx={{ mt: 2 }}>
          <TransactionDetails
            details={[
              { primary: 'Node', secondary: details.address },
              { primary: 'Amount', secondary: `${details.amount + MAJOR_CURRENCY}` },
            ]}
          />
        </Box>
      )}
    </>
  )
}
