import * as Yup from 'yup';
import { isGreaterThan, isLessThan } from 'src/utils';

export const inputValidationSchema = Yup.object().shape({
  profitMargin: Yup.string()
    .required()
    .test('Profit margin must be a number between 0 and 100', (value) => {
      const stringValueToNumber = Math.round(Number(value));
      if (stringValueToNumber && isGreaterThan(stringValueToNumber, -1) && isLessThan(stringValueToNumber, 101))
        return true;
      return false;
    }),
  uptime: Yup.string()
    .required()
    .test('Uptime must be a number between 0 and 100', (value) => {
      const stringValueToNumber = Math.round(Number(value));
      if (stringValueToNumber && isGreaterThan(stringValueToNumber, 0)) return true;
      return false;
    }),
  bond: Yup.string()
    .required()
    .test('Bond must be a valid number', (value) => {
      if (Number(value)) return true;
      return false;
    }),
  delegations: Yup.string()
    .required()
    .test('Delegations must be a valid number', (value) => {
      if (Number(value)) return true;
      return false;
    }),
  operatorCost: Yup.string()
    .required()
    .test('Operator cost must be a valid number', (value) => {
      if (Number(value)) return true;
      return false;
    }),
});
