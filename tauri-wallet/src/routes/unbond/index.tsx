import React from 'react'
import { Layout, NymCard, Page } from '../../components'
import { UnbondForm } from './UnbondForm'

export const Unbond = () => {
  return (
    <Page>
      <Layout>
        <NymCard
          title="Unbond"
          subheader="Unbond a mixnode or gateway"
          noPadding
        >
          <UnbondForm />
        </NymCard>
      </Layout>
    </Page>
  )
}
