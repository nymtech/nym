'use client'

import * as React from 'react';
import { ApiState } from '@/app/typeDefs/explorer-api';

/**
 * Custom hook to get data from the API by passing an id to a delegate method that fetches the data asynchronously
 * @param id                The id to fetch
 * @param fn                Delegate the fetching to this method (must take `(id: string)` as a parameter)
 * @param errorMessage      A static error message, to use when no dynamic error message is returned
 */
export const useApiState = <T>(
  id: string,
  fn: (argId: string) => Promise<T>,
  errorMessage: string,
): [ApiState<T> | undefined, () => Promise<ApiState<T>>, () => void] => {
  // stores the state
  const [value, setValue] = React.useState<ApiState<T>>();

  // clear the value
  const clearValueFn = () => setValue(undefined);

  // this provides a method to trigger the delegate to fetch data
  const wrappedFetchFn = React.useCallback(async () => {
    setValue({ isLoading: true });
    try {
      // keep previous state and set to loading
      setValue((prevState) => ({ ...prevState, isLoading: true }));

      // delegate to user function to get data and set if successful
      const data = await fn(id);
      const newValue: ApiState<T> = {
        isLoading: false,
        data,
      };
      setValue(newValue);
      return newValue;
    } catch (error) {
      // return the caught error or create a new error with the static error message
      const newValue: ApiState<T> = {
        error: error instanceof Error ? error : new Error(errorMessage),
        isLoading: false,
      };
      setValue(newValue);
      return newValue;
    }
  }, [setValue, fn, id, errorMessage]);
  return [value, wrappedFetchFn, clearValueFn];
};
