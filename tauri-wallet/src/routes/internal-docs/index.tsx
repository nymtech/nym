import React from 'react'
import { NymCard } from '../../components'
import { ApiList } from './ApiList'
import { Layout } from '../../layouts'

export const InternalDocs = () => {
  if (process.env.NODE_ENV == 'development') {
    return (
      <Layout>
        <NymCard title="Docs" subheader="Internal API docs">
          <ApiList />
        </NymCard>
      </Layout>
    )
  }

  return null
}
