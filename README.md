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


## Templating Functions
Zinn provides custom functions for the templating language:
- `cat <s1> <s2>...`: Concatenate all parameters
- `joinlines <var>`: Join lines and connect them with a regular whitespace
- `lst <s1> <s2>...`: Create a space-separated list from all input parameters
- `lst-prefix <prefix> <list>`: Add a prefix to each element in a space-separated list
- `lst-re <list> <pattern> <replacement>`: Apply a regex replacement operation to each item in a space-separated list
- `lst-suffix <prefix> <list>`: Add a suffix to each element in a space-separated list
- `lst-without <list> <remove1> <remove2>...`: Create copy of a space-separated list without certain elements
- `re <base> <pattern> <replacement>`: Apply a regex replacement operation to an input string
- `shell <cmd>`: Create a string from the output of a shell command
- `subst <base> <pattern> <replacement>`: Replace all occurrences of a substring

## Nix Support
If **Nix is installed** and **Flakes are enabled**, it is possible to specify build dependencies in the `nix.packages` field of the Zinnfile.
All jobs are then run inside a environment containing these packages.
Please note however that Flakes are currently an unstable feature in Nix, so this feature should be considered unstable as well.
