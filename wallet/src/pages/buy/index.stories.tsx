import { Tutorial } from '../../components/Buy/Tutorial';
import { MockBuyContextProvider } from '../../context/mocks/buy';

export default {
  title: 'Buy/Page',
  component: Tutorial,
};

export const BuyPage = () => (
  <MockBuyContextProvider>
    <Tutorial />
  </MockBuyContextProvider>
);
