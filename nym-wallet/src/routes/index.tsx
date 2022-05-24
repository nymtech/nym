import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { AppRoutes } from './app';
import { AuthRoutes } from './auth';

export const Routes = () => {
  const { clientDetails } = useContext(AppContext);
  if (!clientDetails) return <AuthRoutes />;
  return <AppRoutes />;
};
