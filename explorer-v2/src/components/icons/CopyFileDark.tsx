import { icons } from "@/utils/getIconByName";
import Image from "next/image";

const CopyFile = ({ className }: { className?: string }) => (
  <Image
    src={icons.copyFileDark}
    alt="Copyfile"
    width={16}
    height={16}
    className={className}
  />
);

export default CopyFile;
