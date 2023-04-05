import { Route } from 'react-router-dom';
import { Register } from 'src/pages/register';
import { CreatePassword } from 'src/pages/register/create-password';
import { SetupComplete } from 'src/pages/setup-complete';

export const RegisterRoutes = (
  <>
    <Route path="/register" element={<Register />} />
    <Route path="/register/create-password" element={<CreatePassword />} />
    <Route path="/regeister/complete" element={<SetupComplete />} />
  </>
);
