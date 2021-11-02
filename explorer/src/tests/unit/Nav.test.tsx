import React, { render } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import { Nav } from '../components/Nav';

describe('Nav', () => {
  beforeEach(() => {
    render(<Nav />);
  });
  it('should render without exploding', () => {
    const { container } = render(<Nav />);
    expect(container.firstChild).toBeInTheDocument();
  });
});
