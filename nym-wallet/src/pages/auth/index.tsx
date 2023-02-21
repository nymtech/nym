import React from 'react';
import { AuthProvider } from 'src/context';
import { AuthRoutes } from 'src/routes/auth';

export const Auth = () => (
  <AuthProvider>
    <AuthRoutes />
  </AuthProvider>
);
