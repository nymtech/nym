import { AppBar, Container, Toolbar } from '@mui/material'
import logo from './images/nym-logo.svg'
import { bgcolor } from '@mui/system'
import { NymThemeProvider } from './theme'

export const App = () => {
  return (
    <NymThemeProvider>
      <Container fixed>
        <AppBar
          sx={{
            bgcolor: '#070B15',
            backgroundImage: 'none',
            boxShadow: 'none',
          }}
        >
          <Container fixed>
            <Toolbar>
              <img src={logo} />
            </Toolbar>
          </Container>
        </AppBar>
      </Container>
    </NymThemeProvider>
  )
}
