import Image from "next/image";

import { icons } from "@/utils/getIconByName";

const ArrowUpRight = () => (
  <Image src={icons.arrowUpRight} alt="Arrow Up Right" width={32} height={32} />
);

export default ArrowUpRight;
