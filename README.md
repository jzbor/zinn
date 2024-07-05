# Zinn
Zinn (from the German "Sinn machen", "to make sense") is a builder similar to [make](https://en.wikipedia.org/wiki/Make_(software)), but based on [YAML](https://yaml.org/) and the [Handlebars](https://handlebarsjs.com/guide/) templating language.

It supports **job parameters**, **file tracking** and a cli that **displays the current progress visually**.
An example can be found in [`./examples/cproj`](./examples/cproj).

*Please note: This is currently an experimental sideproject of myself and is not fit for production. The file format may change at any time.*

## Zinnfile
A Zinnfile describes the jobs that should be run.
The [YAML markup language](https://yaml.org) is used in combination with the [Handlebars templating language](https://handlebarsjs.com/guide/) to describe the jobs that should be run.
By default Zinn looks for the file `zinn.yaml`, but a path can also be specified manually.

A basic Zinnfile for a simple C project might look like this:
```yaml
constants:
  CC: gcc
  CFLAGS: -std=c11 -pedantic -Wall -Werror -D_XOPEN_SOURCE=700
jobs:
  object:
    args: [path]
    inputs: "{{path}}"
    outputs: "{{subst path '.c' '.o'}}"
    run: {{CC}} {{CFLAGS}} -c {{path}}

  binary:
    requires:
      - job: object
        with:
    path: math.c
      - job: object
        with:
    path: output.c
      - job: object
        with:
    path: main.c
    inputs: math.c output.c main.c
    outputs: program
    run: {{CC}} {{CFLAGS}} -o program

  clean:
    run: |
      rm -rf *.o
      rm -rf program

  default:
    requires:
      - job: binary
```

You can find more information on the available options [in the source documentation](`crate::Zinnfile`).

