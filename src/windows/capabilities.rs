use once_cell::sync::Lazy;

static IS_WINE: Lazy<bool> = Lazy::new(crate::windows::hachimi_impl::is_wine);

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FeatureSupport {
    Supported,
    WarnOnly,
}

pub fn is_wine() -> bool {
    *IS_WINE
}

pub fn supports_smtc() -> bool {
    !is_wine()
}

pub fn supports_taskbar_progress() -> bool {
    !is_wine()
}

pub fn supports_scheduled_toasts() -> bool {
    !is_wine()
}

pub fn self_update_support() -> FeatureSupport {
    if is_wine() {
        FeatureSupport::WarnOnly
    } else {
        FeatureSupport::Supported
    }
}
