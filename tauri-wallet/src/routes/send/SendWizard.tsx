import React, { useState } from 'react'
import { Button, Step, StepLabel, Stepper, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { SendForm } from './SendForm'
import { SendReview } from './SendReview'
import { SendConfirmation } from './SendConfirmation'
import { SendError } from './SendError'

export const SendWizard = () => {
  const [activeStep, setActiveStep] = useState(0)
  const [toAddress, setToAddress] = useState('')
  const [sendAmount, setSendAmount] = useState('')

  const steps = ['Enter address', 'Review and send', 'Await confirmation']
  const theme: Theme = useTheme()

  const handleNextStep = () =>
    setActiveStep((s) => (s + 1 < steps.length ? s + 1 : s))

  const handlePreviousStep = () =>
    setActiveStep((s) => (s - 1 >= 0 ? s - 1 : s))

  const handleFinish = () => {
    setActiveStep(0)
    setSendAmount('')
    setToAddress('')
  }

  return (
    <div>
      <Stepper
        activeStep={activeStep}
        style={{ background: theme.palette.grey[50] }}
      >
        {steps.map((s, i) => (
          <Step key={i}>
            <StepLabel>{s}</StepLabel>
          </Step>
        ))}
      </Stepper>
      <div
        style={{
          minHeight: 300,
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
        }}
      >
        {activeStep === 0 ? (
          <SendForm
            updateRecipAddress={(address) => setToAddress(address)}
            updateAmount={(amount) => setSendAmount(amount)}
            formData={{ sendAmount, toAddress }}
          />
        ) : activeStep === 1 ? (
          <SendReview recipientAddress={toAddress} amount={sendAmount} />
        ) : sendAmount === 'fail' ? (
          <SendError onFinish={() => setActiveStep((s) => s + 1)} />
        ) : (
          <SendConfirmation
            amount={sendAmount}
            recipient={toAddress}
            onFinish={() => setActiveStep((s) => s + 1)}
          />
        )}
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
        }}
      >
        {activeStep === 1 && (
          <Button
            disableElevation
            style={{ marginRight: theme.spacing(1) }}
            onClick={handlePreviousStep}
          >
            Back
          </Button>
        )}
        <Button
          variant={activeStep > 0 ? 'contained' : 'text'}
          color={activeStep > 0 ? 'primary' : 'default'}
          disableElevation
          onClick={activeStep === 3 ? handleFinish : handleNextStep}
          disabled={!(toAddress.length > 0 && sendAmount.length > 0)}
        >
          {activeStep === 1
            ? 'Send'
            : activeStep === steps.length
            ? 'Finish'
            : 'Next'}
        </Button>
      </div>
    </div>
  )
}
