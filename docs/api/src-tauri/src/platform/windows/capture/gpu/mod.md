# src-tauri/src/platform/windows/capture/gpu/mod.rs

MODULE OVERVIEW: The default Windows recording path. WGC hands each frame's D3D11 surface straight to the Media Foundation `VideoEncoder` with no GPU->CPU readback, which is what cures game-capture lag; the recorded `video.mp4` is the raw full-res intermediate the export re-composites, and the per-frame capture times collected here become `sync.json`. Five files, one responsibility each, because the callback, the encoder settings, the mid-take handover and the resize fit all grew past one file: `record` starts and stops a capture, `frames` is the callback that encodes, `restart` moves a live encode onto another target, and `fit` + `chain` scale a resized capture into the encoder's fixed canvas.

Declaration-only. Everything below is reached as `gpu::record::*`, `gpu::frames::*` and so on; `capture/mod.rs` names only `record` and `frames`.

## record

Capture lifecycle and encoder settings. Key items: `GpuRecorder` (control handle + shared frame timestamps), `GpuStart` (its start config), `start_capture` (resolves a `TargetId` to a monitor or window and launches), `target_bitrate` / `video_settings` / `encoder`.

## frames

The WGC frame callback. Key items: `Cap` (the `GraphicsCaptureApiHandler` - builds the encoder from the first frame's own size, rebases each frame's PTS onto the recording clock, fits a later-resized frame into that fixed size, and reports an OS-closed capture), `CapFlags`, `EncoderSpec`, `FrameTimes`, `SizeHook`, `record_if_encoded`.

## restart

Moving a running capture onto another display or window mid-take without restarting the encoder. Key items: `EncoderSeed`, `Cap::take_seed`, `GpuRecorder::restart`.

## fit

The resize fit: one persistent D3D11 canvas at the encoder's size, with the D3D11 video processor scaling every differently-sized capture frame into it. Key items: `FrameFit`, `Scaler`, `texture`, `RATE`.

## chain

The video-processor objects for one source size: the processor, its input and output views and the staging texture, configured with the letterbox rect `session::record::frame_fit::letterbox` computes. Key item: `Chain`.

### Used by

- `platform/windows/capture/mod.rs` - `start_video` and `VideoSink::switch` drive `record` and `restart`; `VideoSink::Dead` holds a `frames::FrameTimes`.
- `session/record/switch_display.rs` - through the `session::record::gpu_frames` re-export, for `SizeHook`.
