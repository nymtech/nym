import React from 'react';

export const HelpImage = ({ img, imageDescription }: { img: string; imageDescription: string }) => (
  <img src={img} alt={imageDescription} width="100%" />
);
