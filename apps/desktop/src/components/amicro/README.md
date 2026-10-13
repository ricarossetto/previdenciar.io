# Adapted Amicro motion primitives

FadeUp, TiltCard and spring presets were adapted from the Amicro project (Subhan-code/Amicro--Micro-transitions-, upstream commit dc2a183f3a5e6e0ea1b0d75530ac084e3a4e0bd2). The MIT notice is preserved in LICENSE beside this file.

This public tree includes only the adapted primitives used by the app, not the upstream website, backend, analytics or payment components. The runtime imports motion/react from the pinned motion dependency.

Tilting is limited to two degrees and mouse input. Reduced-motion preferences are respected. Tables, financial values and PDF rendering do not use decorative 3D effects. Route transitions do not replay on every form edit.
