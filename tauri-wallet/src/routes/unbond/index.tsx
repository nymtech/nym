import React from 'react'
import { NymCard } from '../../components'
import { UnbondForm } from './UnbondForm'
import { Layout } from '../../layouts'

export const Unbond = () => {
  return (
    <Layout>
      <NymCard title="Unbond" subheader="Unbond a mixnode or gateway" noPadding>
        <UnbondForm />
      </NymCard>
    </Layout>
  )
}
