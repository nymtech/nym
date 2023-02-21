import React from 'react';

export const HelpImage = ({ img, imageDescription }: { img: any; imageDescription: string }) => (
  <img src={img} alt={imageDescription} />
);
