import React from 'react'
import { DelegateForm } from './DelegateForm'
import { NymCard } from '../../components'
import { Layout } from '../../layouts'

export const Delegate = () => {
  return (
    <Layout>
      <NymCard
        title="Delegate"
        subheader="Delegate to mixnode or gateway"
        noPadding
      >
        <DelegateForm />
      </NymCard>
    </Layout>
  )
}
