import Image from "next/image";

import { images } from "@/utils/getImageByName";

export const BlogArticleImage = ({ imageName }: { imageName: string }): JSX.Element => {

  const image = imageName as keyof typeof images;

  // ✅ Get the image source dynamically
  const imageSrc = images[image] || "/images/placeholder.webp";

  return (
        <Image
            src={imageSrc}
            alt={imageName}
            width={120}
            height={60}
            sizes="100vw"
            style={{
            width: "100%",
            height: "auto",
            }}
        />
    );
};
