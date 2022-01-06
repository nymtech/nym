import * as Yup from 'yup'

export const validationSchema = Yup.object({
  profitMarginPercent: Yup.number().typeError('profit margin percent must be a number').min(0).max(100).required(),
})
