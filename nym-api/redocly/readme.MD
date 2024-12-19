# Test / validate OpenAPI spec

`redocly` CLI is an [OpenAPI linter][docs] that enforces good practices by
checking whether a series of lints are applied to your OpenAPI spec.

## Install

You need `npm` and `npx` ([official instructions][instructions])

## Run

```
./redocly.sh
```

## Configuration

- redocly.yaml is the main [config file](https://redocly.com/docs/redoc/config)

## Ignore file

- specifies lints to ignore (some lints may be false alarms/not applicable)
- if you want to add current CLI warnings to an ignore file, run redocly CLI
  with `--generate-ignore-file`

[docs]: https://redocly.com/docs/redoc
[instructions]: https://redocly.com/docs/cli/installation
