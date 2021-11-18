import React from 'react'
import MainNav from '../components/MainNav'
import NodeDelegation from '../components/delegate/NodeDelegation'
import { Layout, NymCard } from '../components'

const DelegateStake = () => {
  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Delegate" subheader="Delegate to Mixnode">
          <NodeDelegation />
        </NymCard>
      </Layout>
    </>
  )
}

export default DelegateStake
