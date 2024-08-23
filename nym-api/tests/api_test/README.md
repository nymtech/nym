# Requirements

Install python 3.10, make sure it's in your PATH.

- Ubuntu: using `apt`
- MacOS: using [official installer][mac-install]

# Managing dependencies

Deps are managed using [pip-tools][pip-tools]

NEVER manually edit `requirements.txt`: it's maintained by `pip-tools`

Do **everything** in venv:
```bash
source venv/bin/activate
```

If `venv` is active in your shell session, it will look like this;
```bash
(venv) ➜ git:(main) ✗
```

if it's NOT, it'll look something like this

```bash
 ➜  git:(main) ✗
```

## Adding a dependency

- instead, add package name (optionally, with version number) to `requirements.in`
- run `./update_deps.sh`


[pip-tools]: https://goyatg.com/pip-tools/
[mac-install]: https://www.python.org/downloads/macos/
