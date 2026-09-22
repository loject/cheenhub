//! Worker-поток и Windows Graphics Capture objects.

use std::sync::Arc;
use std::time::Instant;

use dioxus::logger::tracing::{debug, info, warn};
use futures_channel::{mpsc, oneshot};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, LPARAM, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
use windows::core::{BOOL, Interface, factory};

use super::{CaptureControl, FRAME_POOL_BUFFER_COUNT, FRAME_REPORT_INTERVAL};

pub(super) enum WorkerEvent {
    /// Источник штатно закрылся.
    Ended,
    /// Worker обнаружил ошибку WGC или D3D.
    Error(String),
}

enum CaptureOutcome {
    ExplicitStop,
    SourceEnded,
    RuntimeError(String),
}

/// Размер первого валидного WGC-кадра.
pub(super) struct StartupInfo {
    /// Ширина native-кадра.
    pub(super) width: u32,
    /// Высота native-кадра.
    pub(super) height: u32,
}

/// Запускает WGC lifecycle внутри dedicated capture thread.
pub(super) fn capture_worker(
    source_id: String,
    control: Arc<CaptureControl>,
    startup_sender: oneshot::Sender<Result<StartupInfo, String>>,
    runtime_sender: mpsc::UnboundedSender<WorkerEvent>,
) {
    let mut startup_sender = Some(startup_sender);
    let _mta = match initialize_mta() {
        Ok(mta) => mta,
        Err(error) => {
            send_startup_error(&mut startup_sender, error);
            return;
        }
    };

    let outcome = run_capture(&source_id, &control, &mut startup_sender);

    info!(source_id = %source_id, "Windows screen capture cleanup completed");

    match outcome {
        Ok(CaptureOutcome::ExplicitStop) => {}
        Ok(CaptureOutcome::SourceEnded) => {
            if startup_sender.is_none() {
                let _ = runtime_sender.unbounded_send(WorkerEvent::Ended);
            }
        }
        Ok(CaptureOutcome::RuntimeError(error)) => {
            if startup_sender.is_none() {
                let _ = runtime_sender.unbounded_send(WorkerEvent::Error(error));
            }
        }
        Err(error) => send_startup_error(&mut startup_sender, error),
    }
}

fn initialize_mta() -> Result<MtaGuard, String> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
        .map_err(|error| format!("не удалось инициализировать WinRT MTA: {error}"))?;
    Ok(MtaGuard)
}

struct MtaGuard;

impl Drop for MtaGuard {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

fn run_capture(
    source_id: &str,
    control: &Arc<CaptureControl>,
    startup_sender: &mut Option<oneshot::Sender<Result<StartupInfo, String>>>,
) -> Result<CaptureOutcome, String> {
    if !GraphicsCaptureSession::IsSupported().map_err(platform_error)? {
        return Err("Windows Graphics Capture не поддерживается этой системой".to_owned());
    }

    let monitor = find_monitor(source_id)?;
    info!(source_id = %source_id, "Windows screen capture source found");
    let (_device, direct3d_device) = create_d3d_device()?;
    let interop: IGraphicsCaptureItemInterop =
        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(platform_error)?;
    let item: GraphicsCaptureItem =
        unsafe { interop.CreateForMonitor(monitor) }.map_err(platform_error)?;
    let initial_size = item.Size().map_err(platform_error)?;
    validate_size(initial_size.Width, initial_size.Height)?;

    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &direct3d_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        FRAME_POOL_BUFFER_COUNT,
        initial_size,
    )
    .map_err(platform_error)?;
    let session = frame_pool
        .CreateCaptureSession(&item)
        .map_err(platform_error)?;
    let frame_control = control.clone();
    let frame_token = frame_pool
        .FrameArrived(&TypedEventHandler::new(move |_, _| {
            frame_control.frame_arrived();
            Ok(())
        }))
        .map_err(platform_error)?;
    let closed_control = control.clone();
    let closed_token = item
        .Closed(&TypedEventHandler::new(move |_, _| {
            closed_control.source_closed();
            Ok(())
        }))
        .map_err(platform_error)?;
    let resources = CaptureResources {
        frame_pool,
        session,
        item,
        frame_token,
        closed_token,
    };
    info!(source_id = %source_id, "WGC capture initialized");
    resources.session.StartCapture().map_err(platform_error)?;
    info!(source_id = %source_id, "WGC capture started");

    let mut last_generation = 0;
    let mut native_size = None;
    let mut first_frame_received = false;
    let capture_started_at = Instant::now();
    let mut frames_total = 0_u64;
    let mut frames_since_report = 0_u64;
    let mut last_report = Instant::now();

    loop {
        let snapshot = control.wait_for_change(last_generation);
        if snapshot.stop_requested {
            return Ok(CaptureOutcome::ExplicitStop);
        }
        if snapshot.source_closed {
            if first_frame_received {
                return Ok(CaptureOutcome::SourceEnded);
            } else {
                return Err("выбранный монитор исчез до получения первого кадра".to_owned());
            }
        }
        if snapshot.frame_generation == last_generation {
            continue;
        }
        last_generation = snapshot.frame_generation;

        let frame = match resources.frame_pool.TryGetNextFrame() {
            Ok(frame) => frame,
            Err(error) => {
                return capture_error(first_frame_received, platform_error(error));
            }
        };
        let content_size = match frame.ContentSize() {
            Ok(size) => size,
            Err(error) => {
                return capture_error(first_frame_received, platform_error(error));
            }
        };
        if let Err(error) = validate_size(content_size.Width, content_size.Height) {
            return capture_error(first_frame_received, error);
        }
        let current_size = match (
            u32::try_from(content_size.Width),
            u32::try_from(content_size.Height),
        ) {
            (Ok(width), Ok(height)) => (width, height),
            (Err(_), _) => return capture_error(first_frame_received, "ширина кадра недопустима"),
            (_, Err(_)) => return capture_error(first_frame_received, "высота кадра недопустима"),
        };
        if native_size.is_some_and(|size| size != current_size) {
            drop(frame);
            info!(
                source_id = %source_id,
                native_width = current_size.0,
                native_height = current_size.1,
                "Windows screen capture size changed, recreating frame pool"
            );
            if let Err(error) = resources
                .frame_pool
                .Recreate(
                    &direct3d_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    FRAME_POOL_BUFFER_COUNT,
                    content_size,
                )
                .map_err(platform_error)
            {
                return capture_error(first_frame_received, error);
            }
        } else {
            drop(frame);
        }
        native_size = Some(current_size);
        frames_total = frames_total.saturating_add(1);
        frames_since_report = frames_since_report.saturating_add(1);

        if !first_frame_received {
            first_frame_received = true;
            if let Some(sender) = startup_sender.take() {
                let _ = sender.send(Ok(StartupInfo {
                    width: current_size.0,
                    height: current_size.1,
                }));
            }
            info!(
                source_id = %source_id,
                native_width = current_size.0,
                native_height = current_size.1,
                startup_elapsed_ms = capture_started_at.elapsed().as_millis(),
                "Windows screen capture first valid frame confirmed"
            );
        }

        if last_report.elapsed() >= FRAME_REPORT_INTERVAL {
            let elapsed_seconds = last_report.elapsed().as_secs_f64().max(f64::EPSILON);
            debug!(
                source_id = %source_id,
                native_width = current_size.0,
                native_height = current_size.1,
                frames_total,
                frames_since_last_report = frames_since_report,
                fps = frames_since_report as f64 / elapsed_seconds,
                "Windows screen capture frame statistics"
            );
            frames_since_report = 0;
            last_report = Instant::now();
        }
    }
}

fn send_startup_error(
    startup_sender: &mut Option<oneshot::Sender<Result<StartupInfo, String>>>,
    error: impl Into<String>,
) {
    if let Some(sender) = startup_sender.take() {
        let _ = sender.send(Err(error.into()));
    }
}

fn capture_error(
    first_frame_received: bool,
    error: impl Into<String>,
) -> Result<CaptureOutcome, String> {
    let error = error.into();
    if first_frame_received {
        Ok(CaptureOutcome::RuntimeError(error))
    } else {
        Err(error)
    }
}

fn validate_size(width: i32, height: i32) -> Result<(), String> {
    if width <= 0 || height <= 0 {
        Err("Windows Graphics Capture вернул пустой размер кадра".to_owned())
    } else {
        Ok(())
    }
}

fn find_monitor(source_id: &str) -> Result<HMONITOR, String> {
    unsafe extern "system" fn callback(
        handle: HMONITOR,
        _device_context: HDC,
        _rect: *mut RECT,
        state: LPARAM,
    ) -> BOOL {
        let state = unsafe { &mut *(state.0 as *mut MonitorSearch) };
        if state.handle.is_none()
            && let Some(id) = monitor_id(handle)
            && id == state.source_id
        {
            state.handle = Some(handle);
        }
        true.into()
    }

    let mut search = MonitorSearch {
        source_id,
        handle: None,
    };
    let success = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM((&mut search as *mut MonitorSearch) as isize),
        )
        .as_bool()
    };
    if !success {
        return Err(platform_error(windows::core::Error::from_win32()));
    }
    search
        .handle
        .ok_or_else(|| format!("выбранный монитор {source_id} больше не найден"))
}

struct MonitorSearch<'a> {
    source_id: &'a str,
    handle: Option<HMONITOR>,
}

fn monitor_id(handle: HMONITOR) -> Option<String> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !unsafe {
        GetMonitorInfoW(
            handle,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
        .as_bool()
    } {
        return None;
    }
    Some(wide_string(&info.szDevice))
}

fn wide_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

fn create_d3d_device() -> Result<(ID3D11Device, IDirect3DDevice), String> {
    let mut device = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    let hardware_result = unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    };
    if let Err(error) = hardware_result {
        warn!(%error, "hardware D3D11 device unavailable, using WARP");
        device = None;
        context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .map_err(platform_error)?;
        }
    }
    let device = device.ok_or_else(|| "D3D11 не вернул устройство".to_owned())?;
    let dxgi_device: IDXGIDevice = device.cast().map_err(platform_error)?;
    let inspectable =
        unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }.map_err(platform_error)?;
    let direct3d_device = inspectable.cast().map_err(platform_error)?;
    Ok((device, direct3d_device))
}

struct CaptureResources {
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    item: GraphicsCaptureItem,
    frame_token: i64,
    closed_token: i64,
}

impl Drop for CaptureResources {
    fn drop(&mut self) {
        if let Err(error) = self.frame_pool.RemoveFrameArrived(self.frame_token) {
            warn!(%error, "не удалось снять обработчик WGC FrameArrived");
        }
        if let Err(error) = self.item.RemoveClosed(self.closed_token) {
            warn!(%error, "не удалось снять обработчик WGC Closed");
        }
        if let Err(error) = self.session.Close() {
            warn!(%error, "не удалось закрыть WGC capture session");
        }
        if let Err(error) = self.frame_pool.Close() {
            warn!(%error, "не удалось закрыть WGC frame pool");
        }
    }
}

fn platform_error(error: windows::core::Error) -> String {
    format!("ошибка Windows Graphics Capture: {error}")
}
