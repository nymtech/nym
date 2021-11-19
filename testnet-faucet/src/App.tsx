import { AppBar, Container, Toolbar } from '@mui/material'
import logo from './images/nym-logo.svg'
import { NymThemeProvider } from './theme'
import { Form } from './components/form'
import { Heading } from './components/heading'
import { GlobalContextProvider } from './context'

export const App = () => {
  return (
    <NymThemeProvider>
      <GlobalContextProvider>
        <AppBar
          position="sticky"
          sx={{
            bgcolor: '#070B15',
            backgroundImage: 'none',
            boxShadow: 'none',
            mt: 5,
          }}
        >
          <Container fixed>
            <Toolbar disableGutters>
              <img src={logo} />
            </Toolbar>
          </Container>
        </AppBar>
        <Container fixed>
          <Heading />
          <Form />
        </Container>
      </GlobalContextProvider>
    </NymThemeProvider>
  )
}
