import React, { useState } from 'react'
import { Layout, Page, NymCard } from '../../components'
import { ApiList } from './ApiList'

export const InternalDocs = () => {
  if (process.env.NODE_ENV == 'development') {
    return (
      <Page>
        <Layout>
          <NymCard title="Docs" subheader="Internal API docs" noPadding>
            <ApiList />
          </NymCard>
        </Layout>
      </Page>
    )
  }
}
