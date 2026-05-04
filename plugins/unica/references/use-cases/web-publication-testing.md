# Web Publication And Testing

## When to use

Use this when the user needs Apache publication of a 1C infobase, publication
status, unpublish/stop operations, or browser-based validation through the 1C
web client.

Do not use this for ordinary source build/update. Run the needed `v8-runner`
runtime operation first, then publish or test through the web skills.

## Primary path

- `web-publish`, `web-info`, `web-stop`, and `web-unpublish` cover Apache publication.
- `web-test` covers browser automation through the 1C web client.
- `v8-runner` covers the build/load/test step before web validation.

## Related references

- `references/specs/web-spec.md`
- `references/use-cases/workspace-runtime.md`
