import { InputEvent } from '../../types';

interface SearchProps {
  value: string;
  onChange: (e: InputEvent) => void;
  placeholder: string;
}

export default function SearchBox({
  value,
  onChange,
  placeholder,
}: SearchProps) {
  return (
    <div className="relative w-full flex flex-row justify-center px-6">
      <input
        type="text"
        id="floating_outlined"
        value={value}
        className="dark:bg-baltic-sea pl-9 dark:placeholder-white border border-gun-powder block px-2.5 pb-4 pt-4 w-full text-sm text-gray-900 bg-transparent rounded-lg border-1 border-gray-300 appearance-none dark:text-white dark:border-gray-600 focus:outline-none focus:ring-0 peer"
        placeholder={placeholder}
        onChange={onChange}
      />
      <span className="font-icon scale-125 pointer-events-none absolute fill-current top-1/2 transform -translate-y-1/2 left-9">
        search
      </span>
      <label
        htmlFor="floating_outlined"
        className="dark:text-white dark:bg-baltic-sea absolute text-sm text-gray-500 dark:text-gray-400 ml-8 duration-300 transform -translate-y-4 scale-75 top-2 z-10 origin-[0] bg-blanc-nacre dark:bg-gray-900 px-2 peer-placeholder-shown:px-2 peer-placeholder-shown:top-2 peer-placeholder-shown:scale-75 peer-placeholder-shown:-translate-y-4 rtl:peer-placeholder-shown:translate-x-1/4 rtl:peer-placeholder-shown:left-auto start-1"
      >
        Search
      </label>
    </div>
  );
}
