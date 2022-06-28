import { number, object } from 'yup';

const getSchema = (currentPm: number) =>
  object().shape({
    profitMargin: number().required('Profit Percentage is required').min(0).max(100).notOneOf([currentPm]),
  });

export default getSchema;
