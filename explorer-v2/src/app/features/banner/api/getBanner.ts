// API
import { client } from "../../../lib/strapiClient";

// Types
import type { Languages } from "../../../i18n";

import type { components } from "@/app/lib/strapi";
// Constants
import { bannerApiPath } from "../../banner/config/constants";

// Fetch footer data
export const getBanner = async (
  locale: Languages
): Promise<{
  id?: number;
  attributes?: components["schemas"]["ExplorerBanner"];
} | null> => {
  const banner = await client.GET(bannerApiPath, {
    params: {
      query: {
        locale,
        // @ts-expect-error - populate is not typed correctly?
        populate: {
          links: {
            populate: "*",
          },
          icon: {
            populate: "*",
          },
        },
      },
    },
  });

  return banner?.data?.data ? banner?.data?.data : null;
};
