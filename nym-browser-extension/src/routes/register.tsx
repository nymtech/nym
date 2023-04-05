import { Route } from 'react-router-dom';
import { Register } from 'src/pages/register';
import { SetupComplete } from 'src/pages/register/complete';
import { CreatePassword } from 'src/pages/register/create-password';

export const RegisterRoutes = (
  <>
    <Route path="/register" element={<Register />} />
    <Route path="/register/create-password" element={<CreatePassword />} />
    <Route path="/register/complete" element={<SetupComplete />} />
  </>
);
