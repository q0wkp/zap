// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#import <AppKit/AppKit.h>

#import "app.h"

NSModalResponse configureAndRunModal(NSAlert *alert, NSApplication *app) {
    alert.showsSuppressionButton = YES;

    // It is generally frowned-upon to be overly assertive about putting our windows
    // in the user's face. However, it is reasonable to do this before showing our modal.
    // If we don't make ourselves the top active app, our modal might show up BEHIND an-
    // other app's window.
    [app activateIgnoringOtherApps:YES];
    NSModalResponse response = [alert runModal];

    return response;
}
