import React, { useContext, useState } from 'react'
import { Box } from '@material-ui/core'
import { SecuritySharp } from '@material-ui/icons'
import { Dashboard } from './Dashboard'
import { Layout } from '../../layouts'
import { Setup } from './Setup'
import { theme } from '../../theme'
import { Loading } from '../../components/Loading'
import { ClientContext } from '../../context/main'

export const Socks5 = () => {
  const [isLoading, setIsLoading] = useState(false)
  const [plan, setPlan] = useState<string>()

  const { handleSetBandwidthLimit } = useContext(ClientContext)

  return (
    <Layout>
      <>
        {isLoading && (
          <Box
            display="flex"
            alignItems="center"
            justifyContent="center"
            padding={theme.spacing(1)}
          >
            <Loading
              size="x-large"
              Icon={<SecuritySharp color="primary" style={{ fontSize: 24 }} />}
            />
          </Box>
        )}

        {!isLoading && !!plan && <Dashboard plan={plan} />}

        {!isLoading && !plan && (
          <Setup
            handleSelectPlan={(plan: string) => {
              setIsLoading(true)
              setTimeout(() => {
                setIsLoading(false)
                setPlan(plan)
                handleSetBandwidthLimit(500)
              }, 2000)
            }}
          />
        )}
      </>
    </Layout>
  )
}
