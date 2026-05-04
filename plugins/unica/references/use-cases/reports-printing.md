# Reports, Printing, SKD, And MXL

## When to use

Use this when the user needs reports, SKD/DCS schemas, tabular document layouts,
print forms, BSP external processing registration, or EPF/ERF publication.

Do not use `operation=load` for `.epf` or `.erf`. External processors and
reports are handled through external source-sets with `build`, `dump`, and
`make`.

## Primary path

- `unica.skd.*` for SKD/DCS schema info, compile, edit, and validation.
- `unica.mxl.*` for MXL info, compile, decompile, and validation.
- `unica.template.*` for adding/removing templates on metadata objects.
- `epf-bsp-init` and `epf-bsp-add-command` for BSP registration code.
- `v8-runner` with `unica.runtime.execute` for EPF/ERF external source-set build/dump/make.

## Related references

- `references/specs/1c-dcs-spec.md`
- `references/specs/skd-dsl-spec.md`
- `references/specs/1c-spreadsheet-spec.md`
- `references/specs/mxl-dsl-spec.md`
- `references/specs/1c-epf-spec.md`
- `references/specs/1c-erf-spec.md`
