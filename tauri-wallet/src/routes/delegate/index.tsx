import React from 'react'
import { DelegateForm } from './_DelegateForm'
import { Layout, NymCard, Page } from '../../components'

export const Delegate = () => {
  return (
    <Page>
      <Layout>
        <NymCard
          title="Delegate"
          subheader="Delegate to mixnode or gateway"
          noPadding
        >
          <DelegateForm />
        </NymCard>
      </Layout>
    </Page>
  )
}
