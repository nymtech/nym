import { useTranslation } from 'react-i18next';

function Settings() {
  const { t } = useTranslation();

  return <div>{t('settings')}</div>;
}

export default Settings;
