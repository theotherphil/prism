# Prism

Dimensions: y, x. Compute at blur_v.x, store at blur_v.x
<br/><br/>
<img src="data/inline.gif" alt="inline blur" width="300" />

Dimensions: y, x. Compute at root, store at root
<br/><br/>
<img src="data/intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: y, x. Compute at blur_v.x, store at root
<br/><br/>
<img src="data/local_intermediate.gif" alt="blur with intermediate" width="500" />

Dimensions: yo, y, x. Compute at blur_v.yo, store at blur_v.yo
<br/><br/>
<img src="data/stripped.gif" alt="blur with striping" width="500" />

Dimension: yo, xo, y, x. Compute at blur_v.xo, store at blur_v.xo
<br/><br/>
<img src="data/tiled.gif" alt="blur with striping" width="500" />