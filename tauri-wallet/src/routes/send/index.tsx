import React from 'react'
import { Layout, NymCard, Page } from '../../components'
import { SendWizard } from './SendWizard'

export const Send = () => {
  return (
    <Page>
      <Layout>
        <NymCard title="Send tokens" noPadding>
          <SendWizard />
        </NymCard>
      </Layout>
    </Page>
  )
}
