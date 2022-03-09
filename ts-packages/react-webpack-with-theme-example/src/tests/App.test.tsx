import React, { render } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import { App } from '../App';

describe('App', () => {
  beforeEach(() => {
    render(<App />);
  });
  it('should render without exploding', () => {
    const { container } = render(<App />);
    expect(container.firstChild).toBeInTheDocument();
  });
});
