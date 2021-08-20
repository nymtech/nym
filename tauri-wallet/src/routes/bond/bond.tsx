import React from 'react'
import { Layout, NymCard, Page } from '../../components'
import { BondNodeForm } from './BondForm'

export const Bond = () => {
  return (
    <Page>
      <Layout>
        <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
          <BondNodeForm />
        </NymCard>
      </Layout>
    </Page>
  )
}
