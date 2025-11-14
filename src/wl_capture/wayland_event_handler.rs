use wayland_client::{
    Dispatch,
    protocol::{
        wl_output::WlOutput,
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
};

pub struct WlHandlerStates {
    wl_display_object: Option<WlOutput>,
    wl_shm_object: Option<WlShm>,
    wl_ext_image_copy_capture_manager: Option<ExtImageCopyCaptureManagerV1>,
}

impl Dispatch<WlRegistry, ()> for WlHandlerStates {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Catch the announcement of global objects on the server
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            // Support for copy capture broadcasted
            if interface == "ext_image_copy_capture_manager_v1" {
                // Depend explicitly on version 1.
                // [TODO]: Support multiple version perhaps?
                if version == 1 {
                    // Assign state object
                    state.wl_ext_image_copy_capture_manager =
                        Some(proxy.bind::<ExtImageCopyCaptureManagerV1, _, _>(
                            name,
                            version,
                            qhandle,
                            (),
                        ))
                }
            }

            // Support for wayland output (aka it can show something)
            if interface == "wl_output" {
                state.wl_display_object =
                    Some(proxy.bind::<WlOutput, _, _>(name, version, qhandle, ()));
            }

            // Support for shared memory buffer (so we can actually read what the compositor captured and pass it around)
            if interface == "wl_shm" {
                state.wl_shm_object = Some(proxy.bind::<WlShm, _, _>(name, version, qhandle, ()))
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for WlHandlerStates {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        todo!()
    }
}

impl Dispatch<WlShm, ()> for WlHandlerStates {
    fn event(
        state: &mut Self,
        proxy: &WlShm,
        event: <WlShm as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        todo!()
    }
}

impl Dispatch<ExtImageCopyCaptureManagerV1, ()> for WlHandlerStates {
    fn event(
        state: &mut Self,
        proxy: &ExtImageCopyCaptureManagerV1,
        event: <ExtImageCopyCaptureManagerV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Empty handler as this object have no event to handle
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for WlHandlerStates {
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

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for WlHandlerStates {
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
