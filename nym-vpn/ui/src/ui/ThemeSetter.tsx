import React from 'react';
import clsx from 'clsx';
import { useMainState } from '../contexts';

export default function ThemeSetter({
  children,
}: {
  children: React.ReactNode;
}) {
  const state = useMainState();

  return (
    <div className={clsx([state.uiMode === 'Dark' && 'dark', 'h-full'])}>
      {children}
    </div>
  );
}
