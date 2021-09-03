import React from 'react'
import { NymCard } from '../../components'
import { UndelegateForm } from './UndelegateForm'
import { Layout } from '../../layouts'

export const Undelegate = () => {
  return (
    <Layout>
      <NymCard
        title="Undelegate"
        subheader="Undelegate from a mixnode or gateway"
        noPadding
      >
        <UndelegateForm />
      </NymCard>
    </Layout>
  )
}
