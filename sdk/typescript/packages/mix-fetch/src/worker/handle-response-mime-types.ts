import type { ResponseBody, ResponseBodyConfigMap, ResponseBodyMethod } from '../types';
import { ResponseBodyConfigMapDefaults } from '../types';

const getContentType = (response?: Response) => {
  if (!response) {
    return undefined;
  }

  // this is what should be returned in the headers
  if (response.headers.has('Content-Type')) {
    return response.headers.get('Content-Type') as string;
  }

  // handle weird servers that use lowercase headers
  if (response.headers.has('content-type')) {
    return response.headers.get('content-type') as string;
  }

  // the Content-Type/content-type header is not part of the response
  return undefined;
};

const doHandleResponseMethod = async (response: Response, method?: ResponseBodyMethod): Promise<ResponseBody> => {
  switch (method) {
    case 'uint8array':
      return {
        uint8array: new Uint8Array(await response.arrayBuffer()),
      };
    case 'json':
    case 'text':
      return { text: await response.text() };
    case 'blob': {
      const blob = await response.blob();
      const blobUrl = URL.createObjectURL(blob);
      return { blobUrl };
    }
    case 'formData': {
      const formData: any = {};
      const data = await response.formData();
      // eslint-disable-next-line no-restricted-syntax
      for (const pair of data.entries()) {
        const [key, value] = pair;
        formData[key] = value;
      }
      return { formData };
    }
    default:
      return {};
  }
};

const testIfIncluded = (value?: string, tests?: Array<string | RegExp>): boolean => {
  if (!tests) {
    return false;
  }
  if (!value) {
    return false;
  }

  for (let i = 0; i < tests.length; i += 1) {
    const test = tests[i];
    if (typeof test === 'string' && value === test) {
      return true;
    }
    if ((test as RegExp).test && (test as RegExp).test(value)) {
      return true;
    }
  }

  // default return is false, because nothing above matched
  return false;
};

export const handleResponseMimeTypes = async (
  response: Response,
  config?: ResponseBodyConfigMap,
): Promise<ResponseBody> => {
  // combine the user supplied config with the default
  const finalConfig: ResponseBodyConfigMap = { ...ResponseBodyConfigMapDefaults, ...config };

  const contentType = getContentType(response);

  // check if the headers say what the content type are, otherwise return the bytes of the response as a blob
  if (!contentType) {
    // no content type, or body, so the response is only the status, e.g. GET
    if (!response.body) {
      return {};
    }

    // handle fallback method
    return doHandleResponseMethod(response, config?.fallback || 'blob');
  }

  if (testIfIncluded(contentType, finalConfig.uint8array)) {
    return doHandleResponseMethod(response, 'uint8array');
  }
  if (testIfIncluded(contentType, finalConfig.json)) {
    return doHandleResponseMethod(response, 'json');
  }
  if (testIfIncluded(contentType, finalConfig.text)) {
    return doHandleResponseMethod(response, 'text');
  }
  if (testIfIncluded(contentType, finalConfig.formData)) {
    return doHandleResponseMethod(response, 'formData');
  }
  if (testIfIncluded(contentType, finalConfig.blob)) {
    return doHandleResponseMethod(response, 'blob');
  }

  return {};
};
