use crate::win_util::FocusedMonitorInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingTarget {
    CaptureHotkey,
    DismissHotkey,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Capture,
    ShowOverlay {
        text: String,
        monitor: Option<FocusedMonitorInfo>,
    },
    CloseOverlay,
    CopyAndClose,
    OpenSettings,
    OpenHudEditor,
    HotkeyRecorded {
        target: RecordingTarget,
        binding: String,
    },
    Quit,
}
