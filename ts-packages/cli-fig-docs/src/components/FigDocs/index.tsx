import { Card, CardContent, Typography, CardActions, Button, Stack, Divider, Box } from '@mui/material';
import * as React from 'react';

export const FigOption = (option: Fig.Option): JSX.Element | null => {
  return (
    <Box marginBottom={2} marginTop={2}>
      <Typography variant="h6" fontWeight={700}>
        <pre style={{ margin: 0 }}>
          <code>{option.name}</code>
        </pre>
      </Typography>
      <Typography color="text.secondary" marginBottom={2}>
        {option.description}
      </Typography>
      {option.args && Array.isArray(option.args) ? (
        <>
          <Typography component="span" marginBottom={2}>
            Args:
          </Typography>
          {option.args.map((arg, i) => (
            <>
              <Typography variant="h6" component="span" key={i}>
                {arg.name} {arg.isOptional}
              </Typography>
              {Array.isArray(option.args) && i < option.args.length - 1 && <Divider />}
            </>
          ))}
        </>
      ) : (
        option.args && (
          <Typography component="span">
            Args: {option.args.name} {option.args.isOptional === true && '--is optional'}
          </Typography>
        )
      )}
    </Box>
  );
};

export const FigSubcommand = (subcommand: Fig.Subcommand): JSX.Element | null => {
  return (
    <Box marginLeft={2} marginTop={2}>
      <Typography variant="h6" fontWeight={700}>
        <pre style={{ margin: 0 }}>
          <code>{subcommand.name}</code>
        </pre>
      </Typography>
      <Typography color="text.secondary">{subcommand.description}</Typography>
      {subcommand.subcommands && (
        <>
          <Typography component="div" margin={2}>
            Subcommands:
          </Typography>
          <Divider />
        </>
      )}
      {subcommand.subcommands
        ? subcommand.subcommands.map((command, i) => {
            return (
              <Box key={i}>
                <FigSubcommand {...command} />
                {i < subcommand.subcommands!.length - 1 && <Divider />}
              </Box>
            );
          })
        : null}
      {subcommand.options && (
        <>
          <Typography component="div" margin={2}>
            Options:
          </Typography>
          <Divider />
        </>
      )}
      {subcommand.options
        ? subcommand.options.map((option, i) => {
            return (
              <Box marginLeft={2} key={i}>
                <FigOption {...option} />
                {subcommand!.options && i < subcommand!.options.length - 1 && <Divider />}
              </Box>
            );
          })
        : null}
    </Box>
  );
};

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
          {subcommand.subcommands && (
            <Typography variant="h6" component="div" marginBottom={4}>
              Subcommands:
            </Typography>
          )}
          {subcommand.subcommands?.map((command, i) => {
            return (
              <Card sx={{ minWidth: 275, marginBottom: 2 }} key={i}>
                <CardContent>
                  <FigSubcommand {...command} />
                  {subcommand!.subcommands && i < subcommand!.subcommands.length - 1 && <Divider />}
                </CardContent>
              </Card>
            );
          })}
          {subcommand.options && (
            <Typography variant="h6" component="div" marginBottom={4}>
              Options:
            </Typography>
          )}
          {subcommand.options &&
            subcommand.options.map((option, i) => {
              return (
                <Card sx={{ minWidth: 275, marginBottom: 2 }} key={i}>
                  <CardContent>
                    <FigOption {...option} />
                    {subcommand!.options && i < subcommand!.options.length - 1 && <Divider />}
                  </CardContent>
                </Card>
              );
            })}
        </div>
      )}
    </>
  );
};
