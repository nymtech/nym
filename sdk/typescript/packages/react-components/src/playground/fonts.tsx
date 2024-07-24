const CONTENT = 'The quick brown fox jumped over the white fence';

const WEIGHTS = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];

export const PlaygroundFonts: FCWithChildren = () => (
  <div style={{ fontFamily: 'Open Sans' }}>
    {WEIGHTS.map((fontWeight) => (
      <div key={`weight-${fontWeight}`}>
        <div style={{ fontWeight, fontSize: '30px' }}>{CONTENT}</div>
        <div>
          <code>Font weight: {fontWeight}</code>
        </div>
        <hr />
      </div>
    ))}
  </div>
);
