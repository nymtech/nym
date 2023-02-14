import * as Yup from 'yup';
import { isGreaterThan, isLessThan } from 'src/utils';

export const inputValidationSchema = Yup.object().shape({
  profitMargin: Yup.string()
    .required('profit margin is a required field')
    .test('Is valid profit margin value', (value, ctx) => {
      const stringValueToNumber = Math.round(Number(value));

      if (isGreaterThan(stringValueToNumber, -1) && isLessThan(stringValueToNumber, 101)) return true;
      return ctx.createError({ message: 'Profit margin must be a number from 0 and 100' });
    }),
  uptime: Yup.string()
    .required()
    .test('Is valid uptime value', (value, ctx) => {
      const stringValueToNumber = Math.round(Number(value));
      if (stringValueToNumber && isGreaterThan(stringValueToNumber, 0) && isLessThan(stringValueToNumber, 101))
        return true;
      return ctx.createError({ message: 'Uptime must be a number between 0 and 100' });
    }),
  bond: Yup.string()
    .required()
    .test('Is valid bond value', (value, ctx) => {
      if (Number(value)) return true;
      return ctx.createError({ message: 'Bond must be a valid number' });
    }),
  delegations: Yup.string()
    .required()
    .test('Is valid delegation value', (value, ctx) => {
      if (Number(value)) return true;
      return ctx.createError({ message: 'Delegations must be a valid number' });
    }),
  operatorCost: Yup.string()
    .required('operator cost is a required field')
    .test('Is valid operator cost value', (value, ctx) => {
      const stringValueToNumber = Math.round(Number(value));

      if (isLessThan(stringValueToNumber, 0))
        return ctx.createError({ message: 'Operator cost must be a valid number' });

      return true;
    }),
});
