// Aggregated libplacebo headers for bindgen. We bind libplacebo's Vulkan
// rendering surface (instance/device, swapchain, renderer, frame import) plus
// the colour-management / scaling / dither parameter structs that give us
// mpv-grade output. The Vulkan handle typedefs are pulled in transitively by
// the `pl_*` Vulkan signatures and converted to/from `ash` handles in Rust.
#include <libplacebo/log.h>
#include <libplacebo/common.h>
#include <libplacebo/colorspace.h>
#include <libplacebo/gpu.h>
#include <libplacebo/gamut_mapping.h>
#include <libplacebo/tone_mapping.h>
#include <libplacebo/dither.h>
#include <libplacebo/filters.h>
#include <libplacebo/swapchain.h>
#include <libplacebo/renderer.h>
#include <libplacebo/options.h>
#include <libplacebo/utils/upload.h>
#include <libplacebo/vulkan.h>
