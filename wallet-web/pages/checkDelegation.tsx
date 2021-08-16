import React from 'react'
import MainNav from '../components/MainNav'
import DelegationCheck from '../components/delegation-check/DelegationCheck'
import { Layout, NymCard } from '../components'

const CheckDelegation = () => {
  return (
    <>
      <MainNav />
      <Layout>
        <NymCard
          title="Check Stake"
          subheader="Check your stake on a mixnode or gateway"
        >
          <DelegationCheck />
        </NymCard>
      </Layout>
    </>
  )
}

export default CheckDelegation
