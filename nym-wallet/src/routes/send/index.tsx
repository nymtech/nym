import React, { useState } from 'react'
import { NymCard } from '../../components'
import { SendWizard } from './SendWizard'
import { Layout } from '../../layouts'
import { env_vars } from '../../context/main'

export const Send = () => {
  return (
    <Layout>
      <NymCard title={`Send ${env_vars.MAJOR_CURRENCY}`} noPadding>
        <SendWizard />
      </NymCard>
    </Layout>
  )
}
