import React from 'react'
import { NymCard } from '../../components'
import { Layout } from '../../layouts'
import { BondForm } from './BondForm'

export const Bond = () => {
  return (
    <Layout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
        <BondForm />
      </NymCard>
    </Layout>
  )
}
