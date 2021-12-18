import React, { useState } from 'react'
import { NymCard } from '../../components'
import { SendWizard } from './SendWizard'
import { Layout } from '../../layouts'
import { MAJOR_CURRENCY } from '../../context/main'

export const Send = () => {
  return (
    <Layout>
      <NymCard title={`Send ${MAJOR_CURRENCY}`} noPadding>
        <SendWizard />
      </NymCard>
    </Layout>
  )
}
