import React, { useContext } from 'react'
import { makeStyles } from '@material-ui/core/styles'
import {
  Card,
  CardMedia,
  CardContent,
  Typography,
  CardHeader,
  IconButton,
} from '@material-ui/core'
import logo from '../images/logo.png'
import { theme } from '../theme'
import { ClientContext } from '../context/main'
import { FileCopy, Refresh } from '@material-ui/icons'

const useStyles = makeStyles(({ spacing }) => ({
  root: {
    margin: spacing(10, 5, 3, 7),
    borderRadius: spacing(2), // 16px
    display: 'flex',
    flexDirection: 'row',
    alignItems: 'center',
    paddingBottom: spacing(2),
    paddingTop: spacing(2),
    minWidth: 300,
    position: 'relative',
  },
}))

export const BalanceCard = React.memo(function BlogCard() {
  const styles = useStyles()
  const { client } = useContext(ClientContext)
  return (
    <Card className={styles.root}>
      <CardContent>
        <BalanceCardField
          primaryText="Balance"
          subText={client.balance}
          Action={
            <IconButton size="small">
              <Refresh fontSize="small" />
            </IconButton>
          }
        />
        <BalanceCardField
          primaryText="Address"
          subText={client.address}
          Action={
            <IconButton size="small">
              <FileCopy fontSize="small" />
            </IconButton>
          }
          lastChild
        />
      </CardContent>
    </Card>
  )
})

const BalanceCardField = ({
  primaryText,
  subText,
  Action,
  lastChild,
}: {
  primaryText: string
  subText: string
  Action?: React.ReactNode
  lastChild?: boolean
}) => {
  return (
    <div style={!lastChild ? { marginBottom: theme.spacing(1) } : {}}>
      <Typography variant="body2" style={{}}>
        {primaryText}
      </Typography>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'ceneter',
        }}
      >
        <Typography
          variant="caption"
          style={{
            wordBreak: 'break-all',
            fontWeight: theme.typography.fontWeightBold,
            color: theme.palette.grey[600],
            marginRight: theme.spacing(1),
          }}
        >
          {subText}
        </Typography>
        {Action}
      </div>
    </div>
  )
}
