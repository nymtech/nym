import React, { useContext, useState } from 'react'
import { useForm, FormProvider } from 'react-hook-form'
import { yupResolver } from '@hookform/resolvers/yup'
import { Box, Button, Step, StepLabel, Stepper } from '@mui/material'
import { SendForm } from './SendForm'
import { SendReview } from './SendReview'
import { SendConfirmation } from './SendConfirmation'
import { ClientContext } from '../../context/main'
import { validationSchema } from './validationSchema'
import { TauriTxResult } from '../../types'
import { majorToMinor, send } from '../../requests'
import { checkHasEnoughFunds } from '../../utils'

const defaultValues = {
  amount: '',
  memo: '',
  to: '',
}

export type TFormData = {
  amount: string
  memo: string
  to: string
  from: string
}

export const SendWizard = () => {
  const [activeStep, setActiveStep] = useState(0)
  const [isLoading, setIsLoading] = useState(false)
  const [requestError, setRequestError] = useState<string>()
  const [confirmedData, setConfirmedData] = useState<TauriTxResult['details']>()

  const { getBalance } = useContext(ClientContext)

  const steps = ['Enter address', 'Review and send', 'Await confirmation']

  const { clientDetails } = useContext(ClientContext)
  const methods = useForm<TFormData>({
    defaultValues: {
      ...defaultValues,
      from: clientDetails?.client_address!,
    },
    resolver: yupResolver(validationSchema),
  })

  const handleNextStep = methods.handleSubmit(() => setActiveStep((s) => s + 1))

  const handlePreviousStep = () => setActiveStep((s) => s - 1)

  const handleFinish = () => {
    methods.reset()
    setIsLoading(false)
    setRequestError(undefined)
    setConfirmedData(undefined)
    setActiveStep(0)
  }

  const handleSend = async () => {
    const formState = methods.getValues()

    const hasEnoughFunds = await checkHasEnoughFunds(formState.amount)
    if (!hasEnoughFunds) {
      methods.setError('amount', {
        message: 'Not enough funds in wallet',
      })
      return handlePreviousStep()
    } else {
      setIsLoading(true)
      setActiveStep((s) => s + 1)
      const amount = await majorToMinor(formState.amount)

      send({
        amount,
        address: formState.to,
        memo: formState.memo,
      })
        .then((res: any) => {
          const { details } = res as TauriTxResult
          setActiveStep((s) => s + 1)
          setConfirmedData({
            ...details,
            amount: { denom: 'Major', amount: formState.amount },
          })
          setIsLoading(false)
          getBalance.fetchBalance()
        })
        .catch((e) => {
          setRequestError(e)
          setIsLoading(false)
          console.log(e)
        })
    }
  }

  return (
    <FormProvider {...methods}>
      <Box sx={{ pt: 3 }}>
        <Stepper
          activeStep={activeStep}
          sx={{
            bgcolor: 'grey[50]',
            pb: 0,
            pt: 0,
          }}
        >
          {steps.map((s, i) => (
            <Step key={i}>
              <StepLabel>{s}</StepLabel>
            </Step>
          ))}
        </Stepper>
        <Box
          sx={{
            minHeight: 300,
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            p: [0, 3],
          }}
        >
          {activeStep === 0 ? (
            <SendForm />
          ) : activeStep === 1 ? (
            <SendReview />
          ) : (
            <SendConfirmation
              data={confirmedData}
              isLoading={isLoading}
              error={requestError}
            />
          )}
        </Box>
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
            borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
            background: (theme) => theme.palette.grey[100],
            p: 2,
          }}
        >
          {activeStep === 1 && (
            <Button
              disableElevation
              sx={{ mr: 1 }}
              onClick={handlePreviousStep}
              data-testid="back-button"
            >
              Back
            </Button>
          )}
          <Button
            variant={activeStep > 0 ? 'contained' : 'text'}
            color={activeStep > 0 ? 'primary' : 'inherit'}
            disableElevation
            data-testid="button"
            onClick={
              activeStep === 0
                ? handleNextStep
                : activeStep === 1
                ? handleSend
                : handleFinish
            }
            disabled={
              !!(
                methods.formState.errors.amount ||
                methods.formState.errors.to ||
                isLoading
              )
            }
          >
            {activeStep === 0 ? 'Next' : activeStep === 1 ? 'Send' : 'Finish'}
          </Button>
        </Box>
      </Box>
    </FormProvider>
  )
}
