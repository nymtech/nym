import React, { useEffect, useState } from 'react'
import { Box } from '@mui/system'
import { Stack } from '@mui/material'
import { WelcomeContent, VerifyMnemonic, MnemonicWords, CreatePassword } from './pages'
import { NymLogo } from '../../components'
import { TMnemonicWords, TPages } from './types'
import { RenderPage } from './components'

const mnemonic =
  'futuristic big receptive caption saw hug odd spoon internal dime bike rake helpless left distribution gusty eyes beg enormous word influence trashy pets curl'

const mnemonicToArray = (mnemonic: string): TMnemonicWords =>
  mnemonic
    .split(' ')
    .reduce((a, c: string, index) => [...a, { name: c, index: index + 1, disabled: false }], [] as TMnemonicWords)

export const Welcome = () => {
  const [page, setPage] = useState<TPages>('welcome')
  const [mnemonicWords, setMnemonicWords] = useState<TMnemonicWords>()

  useEffect(() => {
    const mnemonicArray = mnemonicToArray(mnemonic)
    setMnemonicWords(mnemonicArray)
  }, [])

  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        overflow: 'auto',
        bgcolor: 'nym.background.dark',
      }}
    >
      <Box
        sx={{
          width: '100%',
          display: 'flex',
          justifyContent: 'center',
          margin: 'auto',
        }}
      >
        <Stack spacing={3} alignItems="center" sx={{ width: 1080 }}>
          <NymLogo />
          <RenderPage page={page}>
            <WelcomeContent onComplete={() => setPage('create account')} page="welcome" />
            <MnemonicWords
              mnemonicWords={mnemonicWords}
              onNext={() => setPage('verify mnemonic')}
              page="create account"
            />
            <VerifyMnemonic
              mnemonicWords={mnemonicWords}
              onComplete={() => setPage('create password')}
              page="verify mnemonic"
            />
            <CreatePassword page="create password" />
          </RenderPage>
        </Stack>
      </Box>
    </Box>
  )
}
