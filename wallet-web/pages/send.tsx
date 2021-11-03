import React, { useContext } from 'react'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { coin, Coin, printableCoin } from '@nymproject/nym-validator-client'
import {
  Paper,
  Stepper,
  Step,
  StepLabel,
  Button,
  Typography,
} from '@material-ui/core'
import { theme } from '../lib/theme'
import { Layout, NymCard } from '../components'
import { Review } from '../components/send-funds/Review'
import SendNymForm from '../components/send-funds/SendNymForm'
import Confirmation from '../components/Confirmation'
import MainNav from '../components/MainNav'
import { ValidatorClientContext } from '../contexts/ValidatorClient'
import NoClientError from '../components/NoClientError'
import { UDENOM } from './_app'

const steps = ['Enter addresses', 'Review & send', 'Await confirmation']

export interface SendFundsMsg {
  sender: string
  recipient: string
  coin?: Coin
}

export default function SendFunds() {
  const getStepContent = (step) => {
    switch (step) {
      case 0:
        return (
          <SendNymForm
            address={client.address || ''}
            setFormStatus={setFormStatus}
          />
        )
      case 1:
        return <Review {...transaction} />
      case 2:
        const successMessage = `Funds transfer was complete! - sent ${printableCoin(
          transaction.coin
        )} to ${transaction.recipient}`
        return (
          <Confirmation
            isLoading={isLoading}
            progressMessage="Funds transfer is in progress..."
            successMessage={successMessage}
            failureMessage="Failed to complete the transfer"
            error={sendingError}
          />
        )
      default:
        throw new Error('Unknown step')
    }
  }
  const { client } = useContext(ValidatorClientContext)

  console.log('client in send', client)

  // Here's the React state
  const send: SendFundsMsg = { sender: '', recipient: '', coin: null }
  const [activeStep, setActiveStep] = React.useState(0)
  const [transaction, setTransaction] = React.useState(send)
  const [formFilled, setFormFilled] = React.useState(false)

  const [isLoading, setIsLoading] = React.useState(false)
  const [sendingError, setSendingError] = React.useState(null)

  const setFormStatus = (nonEmpty: boolean) => {
    setFormFilled(nonEmpty)
  }

  const handleNext = (event) => {
    event.preventDefault()
    if (activeStep == 0) {
      console.log('activeStep is 0, handling form')
      try {
        handleForm(event)
        setActiveStep(activeStep + 1)
      } catch (e) {
        // right now just don't do anything.
        // this error can be thrown when a value with more than 6 fractionalDigits is entered
        // ideally it should show an error to the user, but it'd involve some additional
        // work to correctly wire it to the form and I'm not sure it's worth it at this current
        // time
      }
    } else if (activeStep == 1) {
      console.log('activeStep is 1, sending funds')
      setActiveStep(activeStep + 1)
      setIsLoading(true)
      console.log('starting funds transfer')
      sendFunds(transaction)
        .then(() => {
          console.log('funds transfer is finished!')
          setIsLoading(false)
        })
        .catch((err) => {
          setSendingError(err)
          setIsLoading(false)
        })
    } else {
      console.log('resetting the progress')
      setIsLoading(false)
      setSendingError(null)
      setActiveStep(0)
    }
  }

  const handleBack = () => {
    setActiveStep(activeStep - 1)
  }

  const getCoinValue = (raw: string): number => {
    let native = printableBalanceToNative(raw)
    return parseInt(native)
  }

  const handleForm = (event) => {
    event.preventDefault()
    let coinValue = getCoinValue(event.target.amount.value)

    const send: SendFundsMsg = {
      sender: client.address,
      recipient: event.target.recipient.value,
      coin: coin(coinValue, UDENOM),
    }
    console.log('Setting transaction', send)
    setTransaction(send)
  }

  const sendFunds = async (transaction: SendFundsMsg) => {
    console.log(`using the context client, our address is ${client.address}`)
    await client.send(client.address, transaction.recipient, [transaction.coin])
  }

  const checkButtonDisabled = (): boolean => {
    if (activeStep === 0) {
      return !formFilled
      // the form must be filled
    } else if (activeStep === 1) {
      // it should always be enabled
      return false
    } else if (activeStep === 2) {
      // transfer must be completed
      return isLoading
    }

    return false
  }

  const getStepperContent = () => {
    return (
      <Paper style={{ padding: theme.spacing(3) }}>
        <Stepper activeStep={activeStep} style={{ paddingLeft: 0 }}>
          {steps.map((label) => (
            <Step key={label}>
              <StepLabel>{label}</StepLabel>
            </Step>
          ))}
        </Stepper>
        <>
          {activeStep === steps.length ? (
            <>
              <Typography variant="h5" gutterBottom>
                Payment complete.
              </Typography>
              <Typography variant="subtitle1">
                You (<b>{transaction.sender}</b>)
              </Typography>
              <Typography variant="subtitle1">
                have sent <b>{printableCoin(transaction.coin)}</b>
              </Typography>
              <Typography variant="subtitle1">
                to <b>{transaction.recipient}</b>.
              </Typography>
            </>
          ) : (
            <>
              <form onSubmit={handleNext}>
                {getStepContent(activeStep)}
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'flex-end',
                    padding: theme.spacing(1, 0),
                  }}
                >
                  {activeStep !== 0 && (
                    <Button onClick={handleBack}>Back</Button>
                  )}
                  <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    data-testid="button"
                    disabled={checkButtonDisabled()}
                  >
                    {activeStep === 1
                      ? 'Send'
                      : activeStep === steps.length - 1
                      ? 'Send again'
                      : 'Next'}
                  </Button>
                </div>
              </form>
            </>
          )}
        </>
      </Paper>
    )
  }

  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Send Nym">
          {client === null ? <NoClientError /> : getStepperContent()}
        </NymCard>
      </Layout>
    </>
  )
}
