import { useRouteError } from 'react-router-dom';

type routerErrorType = {
  statusText: string;
  message: string;
};

export default function Error() {
  const error: routerErrorType = useRouteError() as unknown as routerErrorType;
  console.error(error);

  return (
    <div id="error-page">
      <h1>Oops!</h1>
      <p>Sorry, an unexpected error has occurred.</p>
      <p>
        <i>{error.statusText || error.message}</i>
      </p>
    </div>
  );
}
