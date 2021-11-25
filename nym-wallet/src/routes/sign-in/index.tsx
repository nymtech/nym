import React, { useContext, useState } from 'react'
import { Box } from '@mui/system'
import { SignInContent } from './sign-in'
import { CreateAccountContent } from './create-account'

export const SignIn = () => {
  const [showCreateAccount, setShowCreateAccount] = useState(false)
  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
      }}
    >
      <Box
        sx={{
          width: 500,
          display: 'flex',
          justifyContent: 'center',
        }}
      >
        {showCreateAccount ? (
          <CreateAccountContent
            showSignIn={() => setShowCreateAccount(false)}
          />
        ) : (
          <SignInContent showCreateAccount={() => setShowCreateAccount(true)} />
        )}
      </Box>
    </Box>
  )
}
