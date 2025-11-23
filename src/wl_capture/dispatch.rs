use std::collections::HashMap;

use wayland_client::{
    Dispatch, WEnum,
    protocol::{
        wl_output::{Mode, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::{Format as ShmFormat, WlShm},
    },
};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
        ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    },
};

/// Create an empty `Dispatch` implementation for the corresponding Wayland Object.
///
/// Example:
/// ```
/// no_event_protocols!(wayland_client::protocol::wl_shm::WlShm);
/// ```
macro_rules! no_event_protocols {
    ( $wl_object_type:ty ) => {
        impl Dispatch<$wl_object_type, ()> for ApplicationState {
            fn event(
                _: &mut Self,
                _: &$wl_object_type,
                _: <$wl_object_type as wayland_client::Proxy>::Event,
                _: &(),
                _: &wayland_client::Connection,
                _: &wayland_client::QueueHandle<Self>,
            ) {
                // no-op
            }
        }
    };
}

pub enum WaylandObjects {
    WlOutput,
    WlShm,
    CopyManager,
    CaptureSource,
}

#[derive(Default)]
pub struct ApplicationState {
    // Shared states
    pub wl_output: Option<WlOutput>,
    pub wl_shm: Option<WlShm>,

    pub copy_capture_source_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    pub copy_capture_manager: Option<ExtImageCopyCaptureManagerV1>,
    pub copy_capture_frame: Option<ExtImageCopyCaptureFrameV1>,
    pub copy_capture_session: Option<ExtImageCopyCaptureSessionV1>,

    pub buffer_type: Option<ShmFormat>,
    pub buffer_width: Option<u32>,
    pub buffer_height: Option<u32>,

    // Protocol-specific state, because data field is immutable!
    // I could use Arc<RefCell<T>> to bypass that but...
    // Registry keys
    pub object_keys: HashMap<u32, WaylandObjects>,

    // BufferConfig keys
    pub buffer_config_builder: BufferConfigBuilder,

    // Timestamp keys
    pub frame_timestamp: FrameTimestamp,

    // Capture config
    pub capture_config: Option<CaptureConfigBuilder>,
}

#[derive(Default)]
pub struct BufferConfigBuilder {
    pub has_wanted_type: bool,
    pub buffer_width: u32,
    pub buffer_height: u32,
}

#[derive(Default)]
pub struct FrameTimestamp {
    pub sec_low: u32,
    pub sec_high: u32,
    pub nsec: u32,
}

#[derive(Default)]
pub struct CaptureConfigBuilder {
    pub capture_width: i32,
    pub capture_height: i32,
}

fn remove_object(state: &mut ApplicationState, name: &u32) {
    if let Some(object_type) = state.object_keys.get(name) {
        match object_type {
            WaylandObjects::WlOutput => {
                if let Some(wl_output) = state.wl_output.as_ref() {
                    wl_output.release();
                    state.wl_output = None;
                };
            }
            WaylandObjects::WlShm => {
                if let Some(wl_shm) = state.wl_shm.as_ref() {
                    wl_shm.release();
                    state.wl_shm = None;
                }
            }
            WaylandObjects::CopyManager => {
                if let Some(copy_manager) = state.copy_capture_manager.as_ref() {
                    copy_manager.destroy();
                    state.copy_capture_manager = None;
                }
            }
            WaylandObjects::CaptureSource => {
                if let Some(capture_source) = state.copy_capture_source_manager.as_ref() {
                    capture_source.destroy();
                    state.copy_capture_source_manager = None;
                }
            }
        }

        state.object_keys.remove(name);
    }
}

impl Dispatch<WlRegistry, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            // A new object is added
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "ext_image_copy_capture_manager_v1" => {
                    if version != 1 {
                        return;
                    }

                    state.copy_capture_manager = Some(proxy.bind(name, version, qhandle, ()));
                    state.object_keys.insert(name, WaylandObjects::CopyManager);
                }
                "wl_output" => {
                    state.wl_output = Some(proxy.bind(name, version, qhandle, ()));
                    state.object_keys.insert(name, WaylandObjects::WlOutput);
                }
                "wl_shm" => {
                    state.wl_shm = Some(proxy.bind(name, version, qhandle, ()));
                    state.object_keys.insert(name, WaylandObjects::WlShm);
                }
                "ext_image_capture_source_v1" => {
                    state.copy_capture_source_manager =
                        Some(proxy.bind(name, version, qhandle, ()));
                    state
                        .object_keys
                        .insert(name, WaylandObjects::CaptureSource);
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                remove_object(state, &name);
            }
            _ => unimplemented!(),
        }
    }
}

impl Dispatch<WlOutput, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_output::Event::Mode {
            flags,
            width,
            height,
            refresh: _,
        } = event
            && let WEnum::Value(Mode::Current) = flags
            && (state.capture_config.is_none())
        {
            state.capture_config = Some(CaptureConfigBuilder {
                capture_width: width,
                capture_height: height,
            });
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        _proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event as SessionEvent;

        fn invalidate_current_buffer_config(state: &mut ApplicationState) {
            if state.buffer_width.is_some() {
                state.buffer_width = None;
                state.buffer_config_builder.buffer_width = 0;
            }

            if state.buffer_height.is_some() {
                state.buffer_height = None;
                state.buffer_config_builder.buffer_height = 0;
            }

            if state.buffer_type.is_some() {
                state.buffer_type = None;
                state.buffer_config_builder.has_wanted_type = false;
            }
        }

        match event {
            SessionEvent::BufferSize { width, height } => {
                invalidate_current_buffer_config(state);
                state.buffer_config_builder.buffer_height = height;
                state.buffer_config_builder.buffer_width = width;
            }
            SessionEvent::ShmFormat { format } => {
                invalidate_current_buffer_config(state);

                // [TODO]: Use preferred codec format.
                // avcodec_get_supported_config give a list of Formats that the codec support
                // If we do not match, fall back to yuv420p or xrgb8888
                // (yuv420p use less bandwidth while xrgb888 is guaranteed to be available)

                if let WEnum::Value(ShmFormat::Yuv420) = format {
                    state.buffer_config_builder.has_wanted_type = true;
                }
            }
            SessionEvent::DmabufDevice { device: _ } => {
                invalidate_current_buffer_config(state);

                // [TODO]: Use DMA-BUF if available
            }
            SessionEvent::DmabufFormat {
                format: _,
                modifiers: _,
            } => {
                invalidate_current_buffer_config(state);

                // [TODO]: Use DMA-BUF if available
            }
            SessionEvent::Done => {
                state.buffer_height = Some(state.buffer_config_builder.buffer_height);
                state.buffer_width = Some(state.buffer_config_builder.buffer_width);

                if state.buffer_config_builder.has_wanted_type {
                    state.buffer_type = Some(ShmFormat::Yuv420);
                } else {
                    panic!();
                }
            }
            SessionEvent::Stopped => {
                state.copy_capture_session = None;
            }
            _ => unimplemented!(),
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        _proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event as FrameEvent;

        match event {
            FrameEvent::Transform { transform: _ } => {
                // [TODO]: figure out the transform. Right now we capture in fullscreen so maybe no transform?
            }
            FrameEvent::Damage {
                x: _,
                y: _,
                width: _,
                height: _,
            } => {
                // Ignore since we dont really need to manage damage. Why does this exist..?
            }
            FrameEvent::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                // Something interesting now!
                state.frame_timestamp.sec_low = tv_sec_lo;
                state.frame_timestamp.sec_high = tv_sec_hi;
                state.frame_timestamp.nsec = tv_nsec;
            }
            FrameEvent::Ready => {
                // The buffer sent to the compositor is ready for reading.
                // Build the message to be sent to the encoder, and destroy this object.

                // [TODO]

                state
                    .copy_capture_frame
                    .as_ref()
                    .expect("CaptureFrame object must exist for this callback to run")
                    .destroy();
            }
            FrameEvent::Failed { reason } => {
                use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason as FrameError;

                if let WEnum::Value(t) = reason {
                    match t {
                        // [TODO]: Handle these errors
                        FrameError::Unknown => todo!(),
                        FrameError::BufferConstraints => todo!(),
                        FrameError::Stopped => unreachable!(),
                        _ => unimplemented!(),
                    }
                } else {
                    // No idea what the reason is, panic!
                    unimplemented!("CaptureFrame failed for unknown reason");
                }
            }
            _ => todo!(),
        }
    }
}

// Wayland Protocols without events to handle
no_event_protocols!(WlShm);
no_event_protocols!(ExtOutputImageCaptureSourceManagerV1);
no_event_protocols!(ExtImageCaptureSourceV1);
no_event_protocols!(ExtImageCopyCaptureManagerV1);
