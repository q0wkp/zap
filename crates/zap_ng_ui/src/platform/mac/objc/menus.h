// SPDX-FileCopyrightText: Copyright (C) 2020-2026 Denver Technologies, Inc.
// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: MIT AND AGPL-3.0-only
// Modified by Tianyu (Sky) Lu in 2026: renamed the Warp and Zap names to ZapNG.

#import <AppKit/AppKit.h>

// ZapNGCustomMenuItemHandler is set as both the target and represented object of NSMenuItem.
// It gives the Rust side a chance to dynamically update menu items, and
// respond to their actions.
@interface ZapNGCustomMenuItemHandler : NSObject <NSMenuItemValidation> {
    void *rustContext;
}

// Init, wrapping a pointer which is significant to Rust.
- (id)initWithContext:(void *)wrapper;

// Action set on menu items.
- (void)itemWasTriggered:(NSMenuItem *)item;

// Called when the menu item needs updating.
- (void)itemNeedsUpdate:(NSMenuItem *)item;

@end
