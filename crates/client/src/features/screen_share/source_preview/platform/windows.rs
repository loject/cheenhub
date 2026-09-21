//! Получение статических превью физических мониторов через Windows Graphics Capture.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::logger::tracing::{info, warn};
use futures_channel::oneshot;
use image::{
    ColorType, ImageEncoder, RgbaImage,
    codecs::png::PngEncoder,
    imageops::{FilterType, resize},
};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, LPARAM, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::{BOOL, Interface, PCWSTR, factory};

use super::super::{MonitorPreview, PreviewLoadError};

const FRAME_TIMEOUT: Duration = Duration::from_secs(3);

impl PreviewLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

fn fit_preview_dimensions(width: u32, height: u32) -> (u32, u32) {
    const MAX_WIDTH: u32 = 640;
    const MAX_HEIGHT: u32 = 360;

    if width == 0 || height == 0 || (width <= MAX_WIDTH && height <= MAX_HEIGHT) {
        return (width, height);
    }

    if u64::from(width) * u64::from(MAX_HEIGHT) > u64::from(height) * u64::from(MAX_WIDTH) {
        let scaled_height =
            (u64::from(height) * u64::from(MAX_WIDTH) + u64::from(width) / 2) / u64::from(width);
        (MAX_WIDTH, scaled_height.max(1) as u32)
    } else {
        let scaled_width =
            (u64::from(width) * u64::from(MAX_HEIGHT) + u64::from(height) / 2) / u64::from(height);
        (scaled_width.max(1) as u32, MAX_HEIGHT)
    }
}

fn bgra_rows_to_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
    row_pitch: usize,
) -> Result<Vec<u8>, PreviewLoadError> {
    let packed_row = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| PreviewLoadError::new("размер кадра не помещается в память"))?;
    let required_len = row_pitch
        .checked_mul(
            usize::try_from(height)
                .map_err(|_| PreviewLoadError::new("высота кадра не поддерживается"))?,
        )
        .ok_or_else(|| PreviewLoadError::new("размер кадра не помещается в память"))?;

    if row_pitch < packed_row || pixels.len() < required_len {
        return Err(PreviewLoadError::new("буфер кадра имеет неверный размер"));
    }

    let mut rgba = Vec::with_capacity(
        packed_row
            .checked_mul(height as usize)
            .ok_or_else(|| PreviewLoadError::new("размер кадра не помещается в память"))?,
    );
    for row in pixels.chunks_exact(row_pitch).take(height as usize) {
        for pixel in row[..packed_row].chunks_exact(4) {
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }

    Ok(rgba)
}

#[derive(Clone)]
struct Monitor {
    handle: HMONITOR,
    id: String,
    display_name: String,
    width: u32,
    height: u32,
    primary: bool,
}

/// Сообщает, что Windows поддерживает выбор физического монитора через превью.
pub const fn selection_available() -> bool {
    true
}

/// Загружает по одному PNG-превью для каждого физического монитора Windows.
pub async fn load_previews() -> Result<Vec<MonitorPreview>, PreviewLoadError> {
    match load_previews_inner().await {
        Ok(previews) => Ok(previews),
        Err(error) => {
            warn!(%error, "не удалось загрузить превью мониторов через Windows Graphics Capture");
            Err(PreviewLoadError::new(
                "Не удалось загрузить изображения экранов. Попробуйте ещё раз.",
            ))
        }
    }
}

async fn load_previews_inner() -> Result<Vec<MonitorPreview>, PreviewLoadError> {
    if !GraphicsCaptureSession::IsSupported().map_err(platform_error)? {
        return Err(PreviewLoadError::new(
            "Windows Graphics Capture не поддерживается этой версией Windows",
        ));
    }

    let monitors = enumerate_monitors()?;
    info!(
        monitor_count = monitors.len(),
        "начата загрузка превью мониторов"
    );
    let (device, context, direct3d_device) = create_d3d_device()?;
    let mut previews = Vec::with_capacity(monitors.len());

    let monitor_count = monitors.len();
    for monitor in monitors {
        match capture_monitor(&monitor, &device, &context, &direct3d_device).await {
            Ok(png_bytes) => previews.push(MonitorPreview {
                id: monitor.id,
                display_name: monitor.display_name,
                width: monitor.width,
                height: monitor.height,
                primary: monitor.primary,
                png_bytes,
            }),
            Err(error) => warn!(
                monitor_id = %monitor.id,
                error = %error,
                "не удалось получить превью монитора"
            ),
        }
    }

    info!(
        preview_count = previews.len(),
        "загрузка превью мониторов завершена"
    );
    if monitor_count > 0 && previews.is_empty() {
        return Err(PreviewLoadError::new(
            "не удалось получить кадр ни с одного обнаруженного монитора",
        ));
    }
    Ok(previews)
}

fn enumerate_monitors() -> Result<Vec<Monitor>, PreviewLoadError> {
    unsafe extern "system" fn callback(
        handle: HMONITOR,
        _device_context: HDC,
        _rect: *mut RECT,
        state: LPARAM,
    ) -> BOOL {
        let monitors = unsafe { &mut *(state.0 as *mut Vec<Monitor>) };
        if let Some(monitor) = unsafe { describe_monitor(handle) } {
            monitors.push(monitor);
        }
        true.into()
    }

    let mut monitors = Vec::new();
    unsafe {
        if !EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM((&mut monitors as *mut Vec<Monitor>) as isize),
        )
        .as_bool()
        {
            return Err(platform_error(windows::core::Error::from_win32()));
        }
    }
    Ok(monitors)
}

unsafe fn describe_monitor(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !unsafe {
        GetMonitorInfoW(
            handle,
            (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>(),
        )
        .as_bool()
    } {
        warn!("Windows не вернула описание обнаруженного монитора");
        return None;
    }

    let device_name = wide_string(&info.szDevice);
    let display_name = display_device_name(&info.szDevice).unwrap_or_else(|| device_name.clone());
    let width =
        u32::try_from(info.monitorInfo.rcMonitor.right - info.monitorInfo.rcMonitor.left).ok()?;
    let height =
        u32::try_from(info.monitorInfo.rcMonitor.bottom - info.monitorInfo.rcMonitor.top).ok()?;

    Some(Monitor {
        handle,
        id: device_name,
        display_name,
        width,
        height,
        primary: info.monitorInfo.dwFlags & 1 != 0,
    })
}

fn display_device_name(device_name: &[u16]) -> Option<String> {
    let mut device = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    unsafe {
        if EnumDisplayDevicesW(PCWSTR(device_name.as_ptr()), 0, &mut device, 0).as_bool() {
            let name = wide_string(&device.DeviceString);
            (!name.is_empty()).then_some(name)
        } else {
            None
        }
    }
}

fn wide_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

fn create_d3d_device()
-> Result<(ID3D11Device, ID3D11DeviceContext, IDirect3DDevice), PreviewLoadError> {
    let mut device = None;
    let mut context = None;
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
        warn!(%error, "аппаратное D3D11-устройство недоступно, используется WARP");
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
    let device = device.ok_or_else(|| PreviewLoadError::new("D3D11 не вернул устройство"))?;
    let context = context.ok_or_else(|| PreviewLoadError::new("D3D11 не вернул контекст"))?;
    let dxgi_device: IDXGIDevice = device.cast().map_err(platform_error)?;
    let inspectable =
        unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }.map_err(platform_error)?;
    let direct3d_device = inspectable.cast().map_err(platform_error)?;
    Ok((device, context, direct3d_device))
}

async fn capture_monitor(
    monitor: &Monitor,
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    direct3d_device: &IDirect3DDevice,
) -> Result<Vec<u8>, PreviewLoadError> {
    let interop: IGraphicsCaptureItemInterop =
        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(platform_error)?;
    let item: GraphicsCaptureItem =
        unsafe { interop.CreateForMonitor(monitor.handle) }.map_err(platform_error)?;
    let size = item.Size().map_err(platform_error)?;
    if size.Width <= 0 || size.Height <= 0 {
        return Err(PreviewLoadError::new(
            "Windows Graphics Capture вернул пустой размер монитора",
        ));
    }
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        direct3d_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        size,
    )
    .map_err(platform_error)?;
    let session = frame_pool
        .CreateCaptureSession(&item)
        .map_err(platform_error)?;
    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let token = frame_pool
        .FrameArrived(&TypedEventHandler::new(move |_, _| {
            if let Ok(mut sender) = sender.lock()
                && let Some(sender) = sender.take()
            {
                let _ = sender.send(());
            }
            Ok(())
        }))
        .map_err(platform_error)?;
    let resources = CaptureResources {
        frame_pool,
        session,
        token,
    };
    resources.session.StartCapture().map_err(platform_error)?;

    tokio::time::timeout(FRAME_TIMEOUT, receiver)
        .await
        .map_err(|_| PreviewLoadError::new("истекло время ожидания кадра монитора"))?
        .map_err(|_| PreviewLoadError::new("ожидание кадра монитора было прервано"))?;
    let frame = resources
        .frame_pool
        .TryGetNextFrame()
        .map_err(platform_error)?;
    let content_size = frame.ContentSize().map_err(platform_error)?;
    let content_width = u32::try_from(content_size.Width)
        .map_err(|_| PreviewLoadError::new("ширина кадра монитора недопустима"))?;
    let content_height = u32::try_from(content_size.Height)
        .map_err(|_| PreviewLoadError::new("высота кадра монитора недопустима"))?;
    if content_width == 0 || content_height == 0 {
        return Err(PreviewLoadError::new(
            "Windows Graphics Capture вернул пустой кадр монитора",
        ));
    }
    let surface = frame.Surface().map_err(platform_error)?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(platform_error)?;
    let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(platform_error)?;
    let frame = read_texture(device, context, &texture, content_width, content_height)?;
    let (preview_width, preview_height) = fit_preview_dimensions(frame.width, frame.height);
    let source = RgbaImage::from_raw(frame.width, frame.height, frame.rgba)
        .ok_or_else(|| PreviewLoadError::new("буфер кадра имеет неверный размер"))?;
    let preview = resize(&source, preview_width, preview_height, FilterType::Triangle);
    encode_png(&preview, preview_width, preview_height)
}

struct CaptureResources {
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    token: i64,
}

impl Drop for CaptureResources {
    fn drop(&mut self) {
        let _ = self.frame_pool.RemoveFrameArrived(self.token);
        let _ = self.session.Close();
        let _ = self.frame_pool.Close();
    }
}

fn read_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    content_width: u32,
    content_height: u32,
) -> Result<CapturedFrame, PreviewLoadError> {
    let mut description = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut description) };
    if content_width > description.Width || content_height > description.Height {
        return Err(PreviewLoadError::new(
            "размер содержимого кадра превышает размер D3D11 texture",
        ));
    }
    description.Usage = D3D11_USAGE_STAGING;
    description.BindFlags = 0;
    description.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    description.MiscFlags = 0;

    let mut staging = None;
    unsafe { device.CreateTexture2D(&description, None, Some(&mut staging)) }
        .map_err(platform_error)?;
    let staging =
        staging.ok_or_else(|| PreviewLoadError::new("D3D11 не создал staging texture"))?;
    let destination: ID3D11Resource = staging.cast().map_err(platform_error)?;
    let source: ID3D11Resource = texture.cast().map_err(platform_error)?;
    unsafe { context.CopyResource(&destination, &source) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(&destination, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .map_err(platform_error)?;
    let byte_count = match (mapped.RowPitch as usize).checked_mul(content_height as usize) {
        Some(byte_count) => byte_count,
        None => {
            unsafe { context.Unmap(&destination, 0) };
            return Err(PreviewLoadError::new("размер кадра не помещается в память"));
        }
    };
    if mapped.pData.is_null() {
        unsafe { context.Unmap(&destination, 0) };
        return Err(PreviewLoadError::new("D3D11 вернул пустой буфер кадра"));
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), byte_count) }.to_vec();
    unsafe { context.Unmap(&destination, 0) };
    let rgba = bgra_rows_to_rgba(
        &bytes,
        content_width,
        content_height,
        mapped.RowPitch as usize,
    )?;
    Ok(CapturedFrame {
        rgba,
        width: content_width,
        height: content_height,
    })
}

struct CapturedFrame {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

fn encode_png(image: &RgbaImage, width: u32, height: u32) -> Result<Vec<u8>, PreviewLoadError> {
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(image.as_raw(), width, height, ColorType::Rgba8.into())
        .map_err(|error| PreviewLoadError::new(format!("не удалось закодировать PNG: {error}")))?;
    Ok(png)
}

fn platform_error(error: windows::core::Error) -> PreviewLoadError {
    PreviewLoadError::new(format!("ошибка Windows Graphics Capture: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{bgra_rows_to_rgba, fit_preview_dimensions};

    #[test]
    fn fits_wide_source_inside_preview_bounds_without_changing_aspect_ratio() {
        assert_eq!(fit_preview_dimensions(3840, 2160), (640, 360));
    }

    #[test]
    fn converts_bgra_rows_with_padding_to_tightly_packed_rgba() {
        let pixels = [
            10, 20, 30, 40, 50, 60, 70, 80, 200, 201, 202, 203, 90, 100, 110, 120, 130, 140, 150,
            160, 204, 205, 206, 207,
        ];

        let rgba =
            bgra_rows_to_rgba(&pixels, 2, 2, 12).expect("валидный кадр должен преобразоваться");

        assert_eq!(
            rgba,
            vec![
                30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120, 150, 140, 130, 160,
            ]
        );
    }
}
