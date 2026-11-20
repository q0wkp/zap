// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#import <Carbon/Carbon.h>

typedef int CGSWindowID;
typedef void* CGSConnectionID;

extern CGSConnectionID CGSDefaultConnectionForThread(void);

// Typedef for the CGSSetWindowBackgroundBlurRadius function, which is a private
// API.
typedef CGError CGSSetWindowBackgroundBlurRadiusFunction(CGSConnectionID cid, CGSWindowID wid,
                                                         NSUInteger blur);

// Returns a function pointer to the private CGSSetWindowBackgroundBlurRadius
// API, which can be used to set the background blur radius for an NSWindow.
CGSSetWindowBackgroundBlurRadiusFunction* GetCGSSetWindowBackgroundBlurRadiusFunction(void);
