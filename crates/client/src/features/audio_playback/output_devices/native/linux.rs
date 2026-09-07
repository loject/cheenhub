//! Пользовательские устройства Linux через PulseAudio API, включая PipeWire Pulse.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};

use dioxus::prelude::{debug, warn};
use libpulse_binding::{
    callbacks::ListResult,
    context::{Context, FlagSet, State},
    mainloop::standard::Mainloop,
};

use super::super::contract::{AudioOutputDevice, AudioOutputDevicesResult};

/// Перечисляет устройства на blocking worker, не блокируя интерфейс.
pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
    let started = Instant::now();
    let devices = match tokio::task::spawn_blocking(enumerate_blocking).await {
        Ok(Ok(devices)) => Some(devices),
        Ok(Err(error)) => {
            warn!(%error, "PulseAudio output device enumeration failed");
            None
        }
        Err(error) => {
            warn!(%error, "PulseAudio output enumeration worker failed");
            None
        }
    };
    debug!(elapsed_ms = %started.elapsed().as_millis(), device_count = devices.as_ref().map_or(0, Vec::len), available = devices.is_some(), "finished PulseAudio output enumeration");
    AudioOutputDevicesResult {
        devices,
        system_managed: false,
        permission_required: false,
    }
}

fn enumerate_blocking() -> Result<Vec<AudioOutputDevice>, String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut mainloop = Mainloop::new().ok_or("cannot create PulseAudio mainloop")?;
    let mut context = Context::new(&mainloop, "CheenHub output devices")
        .ok_or("cannot create PulseAudio context")?;
    let result = enumerate_connected(&mut mainloop, &mut context, deadline);
    context.disconnect();
    result
}

fn enumerate_connected(
    mainloop: &mut Mainloop,
    context: &mut Context,
    deadline: Instant,
) -> Result<Vec<AudioOutputDevice>, String> {
    context
        .connect(None, FlagSet::NOAUTOSPAWN, None)
        .map_err(|error| format!("{error}"))?;
    while context.get_state() != State::Ready {
        iterate(mainloop, context, deadline)?;
    }
    let entries = Rc::new(RefCell::new(Vec::new()));
    let complete = Rc::new(RefCell::new(None));
    let callback_entries = Rc::clone(&entries);
    let callback_complete = Rc::clone(&complete);
    let mut operation = context
        .introspect()
        .get_sink_info_list(move |result| match result {
            ListResult::Item(info) => {
                if let Some(name) = info.name.as_deref() {
                    callback_entries.borrow_mut().push(AudioOutputDevice {
                        device_id: name.to_owned(),
                        label: info
                            .description
                            .as_deref()
                            .filter(|label| !label.trim().is_empty())
                            .unwrap_or("Устройство вывода")
                            .to_owned(),
                    });
                }
            }
            ListResult::End => *callback_complete.borrow_mut() = Some(true),
            ListResult::Error => *callback_complete.borrow_mut() = Some(false),
        });
    while complete.borrow().is_none() {
        if let Err(error) = iterate(mainloop, context, deadline) {
            operation.cancel();
            return Err(error);
        }
    }
    if *complete.borrow() != Some(true) {
        return Err("PulseAudio device list request failed".to_owned());
    }
    let devices = std::mem::take(&mut *entries.borrow_mut());
    Ok(disambiguate(devices))
}

fn iterate(mainloop: &mut Mainloop, context: &Context, deadline: Instant) -> Result<(), String> {
    if Instant::now() >= deadline {
        return Err("PulseAudio device enumeration timed out after 3 seconds".to_owned());
    }
    if matches!(context.get_state(), State::Failed | State::Terminated) {
        return Err(format!(
            "PulseAudio context unavailable: {}",
            context.errno()
        ));
    }
    if !mainloop.iterate(false).is_success() {
        return Err("PulseAudio mainloop iteration failed".to_owned());
    }
    std::thread::sleep(Duration::from_millis(5));
    Ok(())
}

fn disambiguate(mut devices: Vec<AudioOutputDevice>) -> Vec<AudioOutputDevice> {
    devices.sort_by(|left, right| left.device_id.cmp(&right.device_id));
    let mut totals = BTreeMap::new();
    for device in &devices {
        *totals.entry(device.label.clone()).or_insert(0) += 1;
    }
    let mut ordinals = BTreeMap::new();
    for device in &mut devices {
        if totals.get(&device.label).copied().unwrap_or(0) > 1 {
            let ordinal = ordinals.entry(device.label.clone()).or_insert(0);
            *ordinal += 1;
            device.label = format!("{} ({ordinal})", device.label);
        }
    }
    devices.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then(left.device_id.cmp(&right.device_id))
    });
    devices
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;
