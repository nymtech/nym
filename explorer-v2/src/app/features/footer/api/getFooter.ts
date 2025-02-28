// API
import { client } from "../../../lib/strapiClient";

// Types
import type { Languages } from "../../../i18n";

import type { components } from "@/app/lib/strapi";
// Constants
import { footerApiPath } from "../../footer/config/constants";

// Fetch footer data
export const getFooter = async (
  locale: Languages,
): Promise<{
  id?: number;
  attributes?: components["schemas"]["Footer"];
} | null> => {
  const footer = await client.GET(footerApiPath, {
    params: {
      query: {
        locale,
        // @ts-expect-error - populate is not typed correctly?

        populate: {
          linkBlocks: {
            populate: "*",
          },
        },
      },
    },
  });

  return footer?.data?.data ? footer?.data?.data : null;
};
