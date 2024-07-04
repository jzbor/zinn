# Zinn
Zinn (from the German "Sinn machen", "to make sense") is a builder similar to [make](https://en.wikipedia.org/wiki/Make_(software)), but based on [YAML](https://yaml.org/) and the [Handlebars](https://handlebarsjs.com/guide/) templating language.

It supports **job parameters**, **file tracking** and a cli that **displays the current progress visually**.
An example can be found in [`./examples/cproj`](./examples/cproj).

*Please note: This is currently an experimental sideproject of myself and is not fit for production. The file format may change at any time.**

## Goals
- [x] Job dependencies
- [x] Templating via [handlebars](https://docs.rs/handlebars)
	- [x] Global constants
	- [x] Shell primitive
- [x] Job parameters
- [x] Handling of intermediary files
