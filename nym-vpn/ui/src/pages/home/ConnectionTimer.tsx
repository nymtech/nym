import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import dayjs from 'dayjs';
import { useMainState } from '../../contexts';

function ConnectionTimer() {
  const { sessionStartDate } = useMainState();
  const [connectionTime, setConnectionTime] = useState('00:00:00');
  const { t } = useTranslation('home');

  useEffect(() => {
    if (!sessionStartDate) {
      return;
    }

    const elapsed = dayjs.duration(dayjs().diff(sessionStartDate));
    setConnectionTime(elapsed.format('HH:mm:ss'));

    const interval = setInterval(() => {
      const elapsed = dayjs.duration(dayjs().diff(sessionStartDate));
      setConnectionTime(elapsed.format('HH:mm:ss'));
    }, 500);

    return () => {
      clearInterval(interval);
    };
  }, [sessionStartDate]);

  return (
    <div className="flex flex-col items-center gap-2">
      <p className="text-sm font-bold text-dim-gray dark:text-mercury-mist">
        {t('connection-time')}
      </p>
      <p className="text-sm font-bold text-baltic-sea dark:text-flawed-white">
        {connectionTime}
      </p>
    </div>
  );
}

export default ConnectionTimer;
