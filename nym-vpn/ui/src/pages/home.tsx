import { useContext } from 'react';
import { MainStateContext } from '../contexts';

function Home() {
  const state = useContext(MainStateContext);

  return (
    <div>
      <h2>NymVPN</h2>
      connection state: {state.state}
    </div>
  );
}

export default Home;
