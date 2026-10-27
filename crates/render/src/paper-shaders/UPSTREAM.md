# Paper Shaders

Repository: https://github.com/paper-design/shaders
Commit: 1923e9f536ea079e2a95d23291cbfacfd69d37f6

These GLSL files contain the original vertex and liquid-metal fragment shader. The fragment's four shared template strings are expanded verbatim from shader-utils.ts. Rendering formulas are unchanged. Runtime adaptation to WebGPU bindings is in paper_metal/gpu.rs. Image preprocessing in paper_metal/mod.rs ports toProcessedLiquidMetal and its original red/black SOR solver.

Upstream fragment SHA-256 (expanded): d8db52fe5c9fa361ebb6364360a674ce4ee31df2deb0222bf12dd82e5384db7e
Upstream vertex SHA-256: d5452769f5dc08d3a378e77611f42343d78c7962fc7983d9508126dc1b8c65d2
