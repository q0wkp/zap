// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#[allow(deprecated)]
use cocoa::base::id;
use zap_ng_ui::platform::mac::make_nsstring;

use crate::channel::ChannelState;

extern "C" {
    /// ObjC function to create and register the NSServices provider for the
    /// application.
    fn zap_ng_register_services_provider();
}

/// Initializes application services.
pub fn init() {
    unsafe {
        zap_ng_register_services_provider();
    }
}

/// Returns an NSString containing the custom URL scheme that this build of the
/// application will respond to.
///
/// Called synchronously from the NSServices dispatch path in
/// `services.m::forFilesFromPasteboard:performAction:`, which wraps the body in
/// an `@autoreleasepool` block. That ambient pool owns the returned NSString.
#[allow(deprecated)]
#[no_mangle]
extern "C-unwind" fn zap_ng_services_provider_custom_url_scheme() -> id {
    make_nsstring(ChannelState::url_scheme())
}
