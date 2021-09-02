import React from 'react'
import { Layout, NymCard, Page } from '../../components'
import { BondForm } from './BondForm'

export const Bond = () => {
  return (
    <Page>
      <Layout>
        <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
          <BondForm />
        </NymCard>
      </Layout>
    </Page>
  )
}
