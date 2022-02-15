import React, {useContext} from 'react'
import { ArrowForward } from '@mui/icons-material'
import { NymCard } from '../../components'
import { SendWizard } from './SendWizard'
import { Layout } from '../../layouts'
import { ClientContext } from '../../context/main'

export const Send = () => {
  const {currency} = useContext(ClientContext)
  return (
    <Layout>
      <NymCard title={`Send ${currency?.major}`} noPadding Icon={ArrowForward}>
        <SendWizard />
      </NymCard>
    </Layout>
  )
}
