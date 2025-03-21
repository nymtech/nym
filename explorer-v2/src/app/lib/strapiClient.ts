import createClient from "openapi-fetch";
import qs from "qs";
import type { paths } from "./strapi";

if (!process.env.NEXT_PUBLIC_CMS_API_URL) {
  throw new Error(
    "NEXT_PUBLIC_CMS_API_URL environment variable is not defined",
  );
}
if (!process.env.NEXT_PUBLIC_CMS_API_NEXT_REVALIDATE) {
  throw new Error(
    "NEXT_PUBLIC_CMS_API_NEXT_REVALIDATE environment variable is not defined",
  );
}

const client = createClient<paths>({
  baseUrl: process.env.NEXT_PUBLIC_CMS_API_URL,
  headers: {
    Accept: "application/json",
  },
  fetch: (request: unknown) => {
    const req = request as Request;
    const url = new URL(req.url, process.env.NEXT_PUBLIC_CMS_API_URL);

    return fetch(new Request(url, req), {
      next: {
        revalidate: Number(process.env.NEXT_PUBLIC_CMS_API_NEXT_REVALIDATE),
      },
    });
  },
  querySerializer(params) {
    return qs.stringify(params, {
      encodeValuesOnly: true,
    });
  },
});

export { client };
