// API
import { client } from "../../../lib/strapiClient";

// Types
import type { Languages } from "../../../i18n";

// Constants
import { footerApiPath } from "../../footer/config/constants";

// Fetch footer data
export const getFooter = async (locale: Languages) => {
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
