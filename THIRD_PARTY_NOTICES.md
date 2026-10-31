# Third-party notices

## JetBrains IntelliJ Platform icons

The inline SVG icons in `prototype/index.html` identified in `docs/icons-used.md` as `expui/…` originate from JetBrains IntelliJ Community and were obtained from OpenIDEA's vendored icon set.

Copyright 2000–2026 JetBrains s.r.o. and contributors. Licensed under the Apache License, Version 2.0. A copy is included at `docs/licenses/Apache-2.0.txt`. Some original SVG headers specify Copyright 2000–2022 JetBrains s.r.o. and contributors.

Sources:
- https://github.com/JetBrains/intellij-community
- https://github.com/l0ng-ai/openidea/tree/main/src/ui/expui
- https://github.com/l0ng-ai/openidea/blob/main/THIRD_PARTY_LICENSES.md

Changes: SVGs were embedded as HTML symbols, wrapper attributes adjusted, and standard monochrome grey colors changed to `currentColor`. Artwork paths were preserved.

## OpenIDEA icons

The Rust / Cargo file-type icons and main-menu icon listed as `openidea/…` in `docs/icons-used.md` come from OpenIDEA, licensed under Apache-2.0. Copyright OpenIDEA contributors.

Source: https://github.com/l0ng-ai/openidea/tree/main/src/ui/openidea

Changes: embedded as HTML symbols with wrapper styling adapted.

The UI layout was informed by IDEA New UI and OpenIDEA's documented layout; ZedX is an independent project and is not affiliated with JetBrains or OpenIDEA.

## Native ZedX exports

`assets/icons/zedx/` exports the same 39 prototype glyphs in light/dark variants. Artwork paths are preserved; wrapper styling and `currentColor` values are resolved per appearance for native multicolor rendering. The source inventory and Apache-2.0 notices above apply. `zedx/icon-manifest.json` maps native asset paths.
