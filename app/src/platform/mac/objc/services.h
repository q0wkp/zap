// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

// Our class for handling NSServices messages.
@interface ZapNGServicesProvider : NSObject
@end

// Functions implemented in Rust.
id zap_ng_services_provider_custom_url_scheme();
void zap_ng_app_open_urls(id app, id urls);
