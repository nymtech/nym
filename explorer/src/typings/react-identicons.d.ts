declare module 'react-identicons' {
  import * as React from 'react';

  interface IdenticonProps {
    string: string;
    size?: number;
    padding?: number;
    bg?: string;
    fg?: string;
    palette?: string[];
    count?: number;
    // getColor: Function;
  }

  declare function Identicon(
    props: IdenticonProps,
  ): React.ReactElement<IdenticonProps>;

  export default Identicon;
}
