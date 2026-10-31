# ZedX fork maintenance

- Put substantial UI changes in new `zedx_*` files or new ZedX modules. Keep the original upstream implementation available and select the replacement through a small, documented entry point.
- Reuse upstream editor, project, Panel and Dock behavior. Avoid copying large files when a new child module can access the existing implementation.
- Record every upstream integration seam and the base commit in `zedx/UPSTREAM.md`.
- Preserve the HTML prototype and design documents as references. Native implementation is authorized.
- Do not commit or push without an explicit user request.
