import * as React from 'react';
import { ApiState } from '../typeDefs/explorer-api';

type WrappedApiFn<T> = () => Promise<ApiState<T>>;

export const useApiState = <T>(
  // eslint-disable-next-line @typescript-eslint/ban-types
  fn: Function,
  errorMessage: string,
): [ApiState<T>, WrappedApiFn<T>] => {
  const [value, setValue] = React.useState<ApiState<T>>();
  const wrappedFn = React.useCallback(async () => {
    try {
      setValue((prevState) => ({ ...prevState, isLoading: true }));
      const data = await fn();
      setValue({
        isLoading: false,
        data,
      });
      return data;
    } catch (error) {
      setValue({
        error: error instanceof Error ? error : new Error(errorMessage),
        isLoading: false,
      });
      return undefined;
    }
  }, [setValue, fn]);
  return [value || { isLoading: true }, wrappedFn];
};
