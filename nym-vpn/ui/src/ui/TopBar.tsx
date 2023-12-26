import { useEffect, useMemo, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { routes } from '../constants';
import { Routes } from '../types';

type NavLocation = {
  title: string;
  leftIcon?: React.ReactNode;
  handleLeftNav?: () => void;
  rightIcon?: React.ReactNode;
  handleRightNav?: () => void;
};

type NavBarData = {
  [key in Routes]: NavLocation;
};

export default function TopBar() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const [currentNavLocation, setCurrentNavLocation] = useState<NavLocation>({
    title: '',
    rightIcon: 'settings',
    handleRightNav: () => {
      navigate(routes.settings);
    },
  });

  const navBarData = useMemo<NavBarData>(() => {
    return {
      '/': {
        title: '',
        rightIcon: 'settings',
        handleRightNav: () => {
          navigate(routes.settings);
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
    setCurrentNavLocation(navBarData[location.pathname as Routes]);
  }, [location.pathname, navBarData]);

  return (
    <nav className="flex flex-row flex-nowrap justify-between items-center shrink-0 bg-white text-baltic-sea dark:bg-baltic-sea-jaguar dark:text-mercury-pinkish h-16 text-xl">
      {currentNavLocation?.leftIcon ? (
        <button className="w-6 mx-4" onClick={currentNavLocation.handleLeftNav}>
          <span className="font-icon dark:text-laughing-jack text-2xl">
            {currentNavLocation.leftIcon}
          </span>
        </button>
      ) : (
        <div className="w-6 mx-4" />
      )}
      <p className="justify-self-center">{currentNavLocation.title}</p>
      {currentNavLocation?.rightIcon ? (
        <button
          className="w-6 mx-4"
          onClick={currentNavLocation.handleRightNav}
        >
          <span className="font-icon dark:text-laughing-jack text-2xl">
            {currentNavLocation.rightIcon}
          </span>
        </button>
      ) : (
        <div className="w-6 mx-4" />
      )}
    </nav>
  );
}
