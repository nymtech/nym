import Image from "next/image";

const CopyFile = ({ className }: { className?: string }) => (
  <Image
    src="/icons/copy-file.svg"
    alt="Copyfile"
    width={16}
    height={16}
    className={className}
  />
);

export default CopyFile;
