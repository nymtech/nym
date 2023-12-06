import React from 'react';

export type InputEvent = React.ChangeEvent<HTMLInputElement>;

export type NodeHop = {
  type: 'entry' | 'exit';
};
