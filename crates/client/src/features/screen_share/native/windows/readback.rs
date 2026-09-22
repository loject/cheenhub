//! Считывание BGRA-кадра WGC из D3D11-текстуры в память CPU.

use windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::core::Interface;

/// Переиспользует staging texture между последовательными кадрами.
pub(super) struct FrameReadback {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    staging: Option<ID3D11Texture2D>,
    staging_size: Option<(u32, u32)>,
    bytes: Vec<u8>,
}

impl FrameReadback {
    /// Создаёт считыватель для D3D11-устройства захвата.
    pub(super) fn new(device: ID3D11Device, context: ID3D11DeviceContext) -> Self {
        Self {
            device,
            context,
            staging: None,
            staging_size: None,
            bytes: Vec::new(),
        }
    }

    /// Возвращает BGRA-строки и их шаг в байтах.
    pub(super) fn read(
        &mut self,
        surface: &IDirect3DSurface,
        width: u32,
        height: u32,
    ) -> Result<(&[u8], usize), String> {
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(platform_error)?;
        let source: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(platform_error)?;
        let mut description = D3D11_TEXTURE2D_DESC::default();
        unsafe { source.GetDesc(&mut description) };
        if description.Width < width || description.Height < height {
            return Err("D3D11 вернул текстуру меньше содержимого экрана".to_owned());
        }
        if self.staging_size != Some((description.Width, description.Height)) {
            description.MipLevels = 1;
            description.ArraySize = 1;
            description.Usage = D3D11_USAGE_STAGING;
            description.BindFlags = 0;
            description.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            description.MiscFlags = 0;
            let mut staging = None;
            unsafe {
                self.device
                    .CreateTexture2D(&description, None, Some(&mut staging))
            }
            .map_err(platform_error)?;
            self.staging = staging;
            self.staging_size = Some((description.Width, description.Height));
        }
        let staging = self
            .staging
            .as_ref()
            .ok_or_else(|| "D3D11 не создал staging texture".to_owned())?;
        let source_resource: ID3D11Resource = source.cast().map_err(platform_error)?;
        let staging_resource: ID3D11Resource = staging.cast().map_err(platform_error)?;
        unsafe {
            self.context
                .CopyResource(&staging_resource, &source_resource)
        };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(&staging_resource, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(platform_error)?;
        let stride = mapped.RowPitch as usize;
        let length = stride.saturating_mul(height as usize);
        if mapped.pData.is_null() || stride < width as usize * 4 || length > isize::MAX as usize {
            unsafe { self.context.Unmap(&staging_resource, 0) };
            return Err("D3D11 вернул некорректный буфер экрана".to_owned());
        }
        let source_bytes = unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), length) };
        self.bytes.clear();
        self.bytes.extend_from_slice(source_bytes);
        unsafe { self.context.Unmap(&staging_resource, 0) };
        Ok((&self.bytes, stride))
    }
}

fn platform_error(error: windows::core::Error) -> String {
    format!("ошибка D3D11 при чтении кадра: {error}")
}
