import * as React from 'react';

export const FigDocs: React.FC<{
  figSpec?: Fig.Spec;
}> = ({ figSpec }) => {
  let subcommand: Fig.Subcommand | undefined;
  if (figSpec) {
    if (typeof figSpec !== 'function') {
      subcommand = figSpec as Fig.Subcommand;
    }
  }

  return (
    <>
      <div>Render some docs here</div>
      {subcommand && (
        <div>
          <div>Name: {subcommand.name}</div>
          <pre>{JSON.stringify(subcommand, null, 2)}</pre>
        </div>
      )}
    </>
  );
};
