import React from 'react'
import MainNav from '../components/MainNav'
import BondNode from '../components/bond/NodeBond'
import { Layout, NymCard } from '../components'

const Bond = () => {
  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Bond a Mixnode" subheader="Bond a mixnode or gateway">
          <BondNode />
        </NymCard>
      </Layout>
    </>
  )
}

export default Bond
