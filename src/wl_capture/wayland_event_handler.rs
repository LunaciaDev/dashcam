use std::collections::HashMap;

use signal_hook::flag;
use wayland_client::{
    Dispatch, WEnum,
    protocol::{
        wl_output::{Mode, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
};

pub enum WaylandObjects {
    WlOutput,
    WlShm,
    CopyManager,
}

#[derive(Default)]
pub struct ApplicationState {
    pub wl_output: Option<WlOutput>,
    pub wl_shm: Option<WlShm>,

    pub capture_width: Option<i32>,
    pub capture_height: Option<i32>,

    pub object_keys: HashMap<u32, WaylandObjects>,

    pub copy_capture_manager: Option<ExtImageCopyCaptureManagerV1>,
    pub copy_capture_frame: Option<ExtImageCopyCaptureFrameV1>,
    pub copy_capture_session: Option<ExtImageCopyCaptureSessionV1>,

    pub frame_width: Option<i32>,
    pub frame_height: Option<i32>,
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
            && (state.capture_height.is_none() && state.capture_width.is_none())
        {
            state.capture_height = Some(height);
            state.capture_width = Some(width);
        }
    }
}

impl Dispatch<WlShm, ()> for ApplicationState {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Shm format is given along with the copy request
    }
}

impl Dispatch<ExtImageCopyCaptureManagerV1, ()> for ApplicationState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureManagerV1,
        _event: <ExtImageCopyCaptureManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Empty handler as this object have no event to handle
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::ShmFormat { format } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::DmabufDevice { device } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::Done => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::Event::Stopped => todo!(),
            _ => todo!(),
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for ApplicationState {
    fn event(
        state: &mut Self,
        proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event::Transform { transform } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event::Damage { x, y, width, height } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event::PresentationTime { tv_sec_hi, tv_sec_lo, tv_nsec } => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event::Ready => todo!(),
            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::Event::Failed { reason } => todo!(),
            _ => todo!(),
        }
    }
}
