import { useTranslation } from 'react-i18next';

type Props = {
  node: 'entry' | 'exit';
};

function NodeLocation({ node }: Props) {
  const { t } = useTranslation();

  return (
    <div>
      {node === 'entry' ? t('fist-hop-selection') : t('last-hop-selection')}
    </div>
  );
}

export default NodeLocation;
