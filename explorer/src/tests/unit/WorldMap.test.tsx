import React, { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/extend-expect';
import { WorldMap } from '../../components/WorldMap';

describe('WorldMap', () => {
  beforeEach(() => {
    render(<WorldMap loading={false} />);
  });
  it('should render without exploding', () => {
    const { container } = render(<WorldMap loading={false} />);
    expect(container.firstChild).toBeInTheDocument();
  });
  it('should render the expected container/child element', () => {
    expect(screen.getByTestId('worldMap__container')).toBeInTheDocument();
  });
  it('should render the title', () => {
    expect(screen.getByText('mix-nodes around the globe')).toBeTruthy();
  });
  it('should render the map/SVG', () => {
    expect(screen.getByTestId('svg')).toBeInTheDocument();
  });
  it('should render map at correct size/dims', () => {
    const expectedWidth = '1000';
    const expectedHeight = '800';
    expect(screen.getByTestId('svg')).toHaveAttribute('width', expectedWidth);
    expect(screen.getByTestId('svg')).toHaveAttribute('height', expectedHeight);
  });
});
