import { AppBar, Container, Toolbar, Typography } from '@mui/material'
import logo from './images/nym-logo.svg'
import { NymThemeProvider } from './theme'
import { Form } from './components/form'
import { Heading } from './components/heading'

export const App = () => {
  return (
    <NymThemeProvider>
      <AppBar
        position="sticky"
        sx={{
          bgcolor: '#070B15',
          backgroundImage: 'none',
          boxShadow: 'none',
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
    </NymThemeProvider>
  )
}
