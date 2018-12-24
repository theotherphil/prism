# Prism

Dimensions: y, x. Compute at blur_v.x, store at blur_v.x
<img src="data/inline.gif" alt="inline blur" width="300" />

Dimensions: y, x. Compute at root, store at root
<img src="data/intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: y, x. Compute at blur_v.x, store at root
<img src="data/local_intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: yo, y, x. Compute at blur_v.yo, store at blur_v.yo
<img src="data/stripped.gif" alt="blur with striping" width="500" />

Dimension: yo, xo, y, x. Compute at blur_v.xo, store at blur_v.xo
<img src="data/tiled.gif" alt="blur with striping" width="500" />