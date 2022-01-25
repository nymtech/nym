import React from 'react'
import { NymCard } from '../../components'
import { SendWizard } from './SendWizard'
import { Layout } from '../../layouts'
import { MAJOR_CURRENCY } from '../../context/main'
import { ArrowForward } from '@mui/icons-material'

export const Send = () => {
  return (
    <Layout>
      <NymCard title={`Send ${MAJOR_CURRENCY}`} noPadding Icon={ArrowForward}>
        <SendWizard />
      </NymCard>
    </Layout>
  )
}
