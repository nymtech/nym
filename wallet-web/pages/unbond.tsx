import React from 'react'
import { Layout, NymCard } from '../components'
import MainNav from '../components/MainNav'
import UnbondNode from '../components/unbond/UnbondNode'

const Unbond = () => {
  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Unbond" subheader="Unbond a Mixnode or Gateway">
          <UnbondNode />
        </NymCard>
      </Layout>
    </>
  )
}

export default Unbond
