# Downloaded model and runtime provenance

BiRefNet by ZhengPeng, MIT: https://github.com/ZhengPeng7/BiRefNet
Original model card: https://huggingface.co/ZhengPeng7/BiRefNet

GPU weights are the MIT-licensed ONNX Community FP16 conversion of BiRefNet, fixed at revision
`534d3c82d3bb8b2f0867db6dfbc3a525b8e42f67`:
https://huggingface.co/onnx-community/BiRefNet-ONNX/tree/534d3c82d3bb8b2f0867db6dfbc3a525b8e42f67
That repository's FP32 file is byte-identical to the original official epoch-244 ONNX export
used by the CPU fallback. Neither model is a commercial-only or noncommercial replacement.

The accompanying LICENSE contains the original BiRefNet copyright and MIT permission text.
ONNX Runtime and its third-party notices are included unchanged from Microsoft's version-1.22.0
DirectML NuGet package. DirectML-LICENSE.txt is unchanged from Microsoft's version-1.15.4 package
and governs its Windows binary; it is not represented as MIT.

See `docs/birefnet-background-removal.md` for the full pinned download URLs, sizes and SHA-256 values.
Weights and runtime binaries are downloaded on demand, never tracked in this repository.
