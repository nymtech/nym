import { useEffect, useMemo, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import clsx from 'clsx';
import { AppName } from '../constants';

export type Props = {
  children?: React.ReactNode;
};

type Routes = '' | 'settings' | 'entry-node-location' | 'exit-node-location';
type RoutePaths = `/${Routes}`;

type NavLocation = {
  title: string;
  leftIcon?: React.ReactNode;
  handleLeftNav?: () => void;
  rightIcon?: React.ReactNode;
  handleRightNav?: () => void;
};

type NavBarData = {
  [key in RoutePaths]: NavLocation;
};

export default function TopBar() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const [currentNavLocation, setCurrentNavLocation] = useState<NavLocation>({
    title: AppName,
    rightIcon: 'settings',
    handleRightNav: () => {
      navigate('/settings');
    },
  });

  const navBarData = useMemo<NavBarData>(() => {
    return {
      '/': {
        title: AppName,
        rightIcon: 'settings',
        handleRightNav: () => {
          navigate('/settings');
        },
      },
      '/settings': {
        title: t('settings'),
        leftIcon: 'arrow_back',
        handleLeftNav: () => {
          navigate(-1);
        },
      },
      '/entry-node-location': {
        title: t('first-hop-selection'),
        leftIcon: 'arrow_back',
        handleLeftNav: () => {
          navigate(-1);
        },
      },
      '/exit-node-location': {
        title: t('last-hop-selection'),
        leftIcon: 'arrow_back',
        handleLeftNav: () => {
          navigate(-1);
        },
      },
    };
  }, [t, navigate]);

  useEffect(() => {
    setCurrentNavLocation(navBarData[location.pathname as RoutePaths]);
  }, [location.pathname, navBarData]);

  return (
    <nav className="flex dark:bg-baltic-sea-jaguar">
      {currentNavLocation?.leftIcon && (
        <button className={clsx([])} onClick={currentNavLocation.handleLeftNav}>
          <span className="font-icon">{currentNavLocation.leftIcon}</span>
        </button>
      )}
      {currentNavLocation.title}
      {currentNavLocation?.rightIcon && (
        <button
          className={clsx([])}
          onClick={currentNavLocation.handleRightNav}
        >
          <span className="font-icon">{currentNavLocation.rightIcon}</span>
        </button>
      )}
    </nav>
  );
}
