import React from 'react'
import { Layout, NymCard } from '../components'
import MainNav from '../components/MainNav'
import NodeUndelegation from '../components/undelegate/NodeUndelegation'

const UndelegateStake = () => {
  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Undelegate" subheader="Undelegate from a Mixnode">
          <NodeUndelegation />
        </NymCard>
      </Layout>
    </>
  )
}

export default UndelegateStake
