import React from 'react'
import { Layout, NymCard, Page } from '../../components'
import { UndelegateForm } from './UndelegateForm'

export const Undelegate = () => {
  return (
    <Page>
      <Layout>
        <NymCard
          title="Undelegate"
          subheader="Undelegate from a mixnode or gateway"
          noPadding
        >
          <UndelegateForm />
        </NymCard>
      </Layout>
    </Page>
  )
}
