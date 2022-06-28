import { number, object } from 'yup';

const schema = object().shape({
  profitMargin: number().required('Profit Percentage is required').min(0).max(100),
});

export default schema;
