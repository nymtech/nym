import { useMainState } from '../../contexts';
import { QuickConnectPrefix } from '../../constants';

interface QuickConnectProps {
  onClick: (name: string, code: string) => void;
}

export default function QuickConnect({ onClick }: QuickConnectProps) {
  const { defaultNodeLocation } = useMainState();

  return (
    <div
      role="presentation"
      onKeyDown={() =>
        onClick(defaultNodeLocation.name, defaultNodeLocation.code)
      }
      className="flex px-1 flex-row items-center w-full py-8 cursor-pointer"
      onClick={() =>
        onClick(defaultNodeLocation.name, defaultNodeLocation.code)
      }
    >
      <span className="font-icon text-2xl px-4 cursor-pointer">bolt</span>
      <div className="cursor-pointer">{`${QuickConnectPrefix} (${defaultNodeLocation.name})`}</div>
    </div>
  );
}
