# BiRefNet background removal

Windows desktop: select one image layer, then **Design → Custom → Remove background (BiRefNet)**.
The first visible image fill is processed. **Restore original background** restores its original
encoded file. Applying or restoring is one undo step. Existing crop, transforms, colour adjustments
and custom Paper effects remain attached to the layer.

The first use downloads approximately 710 MB of pinned public model/runtime files. CPU fallback
downloads another 973 MB once, when needed. Later runs use
the cache at `%APPDATA%/FigCraft/birefnet`. Images are decoded and processed locally; no image bytes
are uploaded and no API key, paid service or usage allowance is involved. The model computes a
1024 × 1024 soft mask, which is resized to the original image dimensions and multiplied by the
original alpha. Original RGB bytes and embedded ICC profile are preserved in the lossless PNG.
Keeping the output dimensions does not mean the model perfectly resolves every fine detail.
The original encoded image, full-size mask and resulting PNG are kept in `.figcraft` files.
The rendered PNG is also usable by ordinary image exports and other apps without BiRefNet.

Supported input is a still, 8-bit image, at most 256 MB decoded. GIFs and higher bit depths are
rejected explicitly. The original is preserved on cancellation or failure. A changed fill rejects
late results, and switching/closing the source document cancels its job. Inference runs on a worker
thread; Cancel discards the result immediately, but an active native GPU call finishes before the
worker can stop. The next job can start once the previous worker has finished.
The loaded model is reused for consecutive images, then released after 20 seconds idle to return
GPU memory to the canvas and other applications.

DirectML is attempted first, with CPU fallback when GPU initialization or inference fails. Windows 10/11
x64 and a DirectX 12 GPU are required for acceleration. CPU processing can take considerably longer.
Large DirectML graph fusion is disabled to avoid memory spikes and Windows GPU timeouts; individual
operators use the same model and resolution. On machines missing Microsoft's VC++ x64 runtime,
install the [official Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
Mac, Linux and browser inference are not included in this version; existing cutout documents still
open and render there. No Python installation is needed.

## Pinned components and commercial use

BiRefNet code and official model: MIT; copyright and permission text are included in
`third-party/birefnet/LICENSE`. There is no noncommercial model substituted behind this feature.
MIT allows commercial use; rights to the user's source photographs are separate.

| Component | Source | Size | SHA-256 |
|---|---|---:|---|
| BiRefNet general FP16 (GPU, 1024 px) | [ONNX Community, fixed revision 534d3c82](https://huggingface.co/onnx-community/BiRefNet-ONNX/tree/534d3c82d3bb8b2f0867db6dfbc3a525b8e42f67) | 489,666,272 B | `3654c741eb80bd926ada8fed1713b506ccf8d30eb1f6487e87eb9f234f33df09` |
| BiRefNet general, epoch 244 (CPU, 1024 px) | [Official v1 ONNX release](https://github.com/ZhengPeng7/BiRefNet/releases/tag/v1) | 972,666,916 B | `58f621f00f5d756097615970a88a791584600dcf7c45b18a0a6267535a1ebd3c` |
| ONNX Runtime DirectML 1.22.0 | [Microsoft NuGet](https://www.nuget.org/packages/Microsoft.ML.OnnxRuntime.DirectML/1.22.0) | 17,898,472 B | `29f9872d786236b79aa83f94482f3a17c14297e4833768d6d0ed4883ee732e60` |
| DirectML 1.15.4 | [Microsoft NuGet](https://www.nuget.org/packages/Microsoft.AI.DirectML/1.15.4) | 202,292,617 B | `4e7cb7ddce8cf837a7a75dc029209b520ca0101470fcdf275c1f49736a3615b9` |
| Rust ort | [pykeio/ort rc.10](https://github.com/pykeio/ort/tree/v2.0.0-rc.10) | — | Cargo.lock pins the crate checksum |

Downloads are checked against fixed lengths and SHA-256 before use. Only an allowlist of x64 DLLs
and notices is extracted; archive paths are never used as output paths. ONNX Runtime is MIT and
its third-party notices are included. DirectML binaries use Microsoft's DirectML licence, which
permits use and distribution in Windows/Xbox applications; its complete terms are included, rather
than describing those binaries as MIT. Runtime downloads retain their notices in the local cache.
ONNX telemetry is disabled. Model weights are not committed to Git or embedded in the app ZIP.
The GPU model is ONNX Community's MIT-licensed FP16 conversion of the same BiRefNet model. Its
FP32 file has the identical SHA-256 as the official epoch-244 export. FP16 reduces GPU memory use
without reducing the 1024-pixel inference size; rounding can cause small differences in soft alpha.

Manual real-model integration probe (native Windows):

```sh
cargo run -p figcraft --example birefnet_probe -- photo.png cache-directory cutout.png
```

This uses the same download, verification, preprocessing, inference and PNG pipeline as the UI.
